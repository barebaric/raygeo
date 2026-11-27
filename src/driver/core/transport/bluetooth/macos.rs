//! macOS RFCOMM backend.
//!
//! IOBluetooth delivers RFCOMM data through delegate callbacks on the
//! run loop of the thread that opened the channel.  The link therefore
//! owns a dedicated thread with its own autorelease pool and run loop;
//! the async transport only sees the bytes it forwards.

use std::ffi::c_void;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_foundation::{
    NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSObjectProtocol,
    NSRunLoop, NSString,
};
use objc2_io_bluetooth::{
    IOBluetoothDevice, IOBluetoothRFCOMMChannel,
    IOBluetoothRFCOMMChannelDelegate, IOBluetoothSDPUUID,
};
use tokio::sync::mpsc;

use super::{normalize_address, BluetoothDeviceInfo, Link};

const SPP_SERVICE_ID: u16 = 0x1101;

struct MacChannel {
    channel: Retained<IOBluetoothRFCOMMChannel>,
}

// SAFETY: the channel object itself is only ever touched through
// `getMTU` and `writeSync`, both of which IOBluetooth documents as
// callable from any thread (the synchronous calls marshal against
// the channel's run loop instead of touching shared state from the
// caller).  The delegate callbacks run exclusively on the link's
// own run-loop thread and never call back into `MacChannel`, so no
// data race is introduced by moving or sharing the handle.
unsafe impl Send for MacChannel {}
unsafe impl Sync for MacChannel {}

impl MacChannel {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        // SAFETY: plain foreign method call on a retained object
        // (see the Send/Sync invariant above); no pointers passed.
        let mtu = unsafe { self.channel.getMTU() };
        let mtu = if mtu == 0 { 1024 } else { usize::from(mtu) };
        for chunk in data.chunks(mtu) {
            let mut chunk = chunk.to_vec();
            let status = unsafe {
                // SAFETY: the buffer is a live, exclusively owned
                // allocation of exactly `chunk.len()` bytes for the
                // duration of the synchronous call.
                self.channel.writeSync_length(
                    chunk.as_mut_ptr().cast::<c_void>(),
                    chunk.len() as u16,
                )
            };
            if status != 0 {
                return Err(io::Error::other(format!(
                    "IOBluetooth write failed with status {status}"
                )));
            }
        }
        Ok(())
    }
}

struct MacDelegateIvars {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacDelegateIvars]
    struct MacDelegate;

    unsafe impl NSObjectProtocol for MacDelegate {}

    unsafe impl IOBluetoothRFCOMMChannelDelegate for MacDelegate {
        #[unsafe(method(rfcommChannelData:data:length:))]
        fn rfcomm_data(
            &self,
            _channel: Option<&IOBluetoothRFCOMMChannel>,
            data: *mut c_void,
            length: usize,
        ) {
            if data.is_null() || length == 0 {
                return;
            }
            // SAFETY: the callback receives `length` valid bytes in
            // `data`; they are copied into an owned `Vec` immediately
            // and the raw parts are not used afterwards.
            let bytes = unsafe {
                std::slice::from_raw_parts(data.cast::<u8>(), length)
            };
            let _ = self.ivars().tx.send(bytes.to_vec());
        }

        #[unsafe(method(rfcommChannelClosed:))]
        fn rfcomm_closed(&self, _channel: Option<&IOBluetoothRFCOMMChannel>) {
            self.ivars().open.store(false, Ordering::SeqCst);
        }
    }
);

impl MacDelegate {
    fn new(
        tx: mpsc::UnboundedSender<Vec<u8>>,
        open: Arc<AtomicBool>,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(MacDelegateIvars { tx, open });
        // SAFETY: `this` is a fresh, uninitialized allocation handed
        // out by `define_class!`'s alloc path; `init` is the
        // superclass initializer that must run exactly once here.
        unsafe { msg_send![super(this), init] }
    }
}

struct MacLink {
    channel: Arc<MacChannel>,
    stop: Arc<AtomicBool>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for MacLink {
    fn drop(&mut self) {
        self.close();
    }
}

impl Link for MacLink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        self.channel.write(data)
    }

    fn close(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.reader.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

pub(crate) fn connect(
    address: &str,
    channel: u8,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) -> io::Result<Arc<dyn Link>> {
    let address = address.to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let thread_open = open.clone();
    let (result_tx, result_rx) =
        std_mpsc::sync_channel::<Result<Arc<MacChannel>, String>>(1);

    let reader = std::thread::Builder::new()
        .name("raydriver-rfcomm-runloop".to_string())
        .spawn(move || {
            run_loop_thread(
                &address,
                channel,
                tx,
                thread_open,
                thread_stop,
                result_tx,
            );
        })?;

    match result_rx.recv() {
        Ok(Ok(channel)) => {
            open.store(true, Ordering::SeqCst);
            Ok(Arc::new(MacLink {
                channel,
                stop,
                reader: Mutex::new(Some(reader)),
            }))
        }
        Ok(Err(message)) => {
            let _ = reader.join();
            Err(io::Error::other(message))
        }
        Err(_) => {
            let _ = reader.join();
            Err(io::Error::other(
                "IOBluetooth run loop stopped before opening the channel",
            ))
        }
    }
}

fn run_loop_thread(
    address: &str,
    requested_channel: u8,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    result_tx: std_mpsc::SyncSender<Result<Arc<MacChannel>, String>>,
) {
    // SAFETY: creating the pool at the top of the thread; it is
    // drained on every exit path below.
    let pool = unsafe { NSAutoreleasePool::new() };
    let delegate = MacDelegate::new(tx, open.clone());

    let channel = match open_channel(address, requested_channel, &delegate) {
        Ok(channel) => channel,
        Err(message) => {
            let _ = result_tx.send(Err(message));
            unsafe { pool.drain() };
            return;
        }
    };

    let channel = Arc::new(channel);
    if result_tx.send(Ok(channel.clone())).is_err() {
        unsafe { pool.drain() };
        return;
    }

    let run_loop = NSRunLoop::currentRunLoop();
    while !stop.load(Ordering::SeqCst) && open.load(Ordering::SeqCst) {
        let limit = NSDate::dateWithTimeIntervalSinceNow(0.1);
        unsafe {
            run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &limit);
        }
    }

    // SAFETY: the channel is retained until here; closing it before
    // draining the pool releases any objects the callbacks enqueued.
    unsafe {
        channel.channel.closeChannel();
        pool.drain();
    }
    open.store(false, Ordering::SeqCst);
}

fn open_channel(
    address: &str,
    requested_channel: u8,
    delegate: &Retained<MacDelegate>,
) -> Result<MacChannel, String> {
    let ns_address = NSString::from_str(address);
    // SAFETY: foreign constructor returning a retained object; the
    // address string is retained for the call.
    let device = unsafe {
        IOBluetoothDevice::deviceWithAddressString(Some(&ns_address))
    }
    .ok_or_else(|| format!("Bluetooth device {address} is not known"))?;

    let channel_id = sdp_channel(&device).unwrap_or(requested_channel);
    let mut channel: Option<Retained<IOBluetoothRFCOMMChannel>> = None;
    // SAFETY: the delegate pointer stays valid for the whole
    // channel lifetime: `run_loop_thread` owns the `Retained` and
    // closes the channel before dropping it (the run loop exits and
    // drains after the channel is closed).
    let delegate = Retained::as_ptr(delegate).cast::<AnyObject>();
    let status = unsafe {
        device.openRFCOMMChannelSync_withChannelID_delegate(
            Some(&mut channel),
            channel_id,
            Some(&*delegate),
        )
    };
    if status != 0 {
        return Err(format!(
            "IOBluetooth could not open RFCOMM channel {channel_id} \
             (status {status})"
        ));
    }
    let channel = channel
        .ok_or_else(|| "IOBluetooth returned no RFCOMM channel".to_string())?;
    Ok(MacChannel { channel })
}

fn sdp_channel(device: &IOBluetoothDevice) -> Option<u8> {
    // SAFETY: plain foreign calls on retained objects; `channel` is
    // an initialized out-parameter owned by this frame.
    let uuid = unsafe { IOBluetoothSDPUUID::uuid16(SPP_SERVICE_ID) }?;
    let record = unsafe { device.getServiceRecordForUUID(Some(&uuid)) }?;
    let mut channel: u8 = 0;
    // SAFETY: as above; the out-pointer receives the channel number.
    if unsafe { record.getRFCOMMChannelID(&mut channel) } == 0 {
        Some(channel)
    } else {
        None
    }
}

pub(crate) fn list_paired_devices() -> io::Result<Vec<BluetoothDeviceInfo>> {
    let pool = unsafe { NSAutoreleasePool::new() };
    let devices = unsafe { IOBluetoothDevice::pairedDevices() };
    let mut out = Vec::new();
    if let Some(devices) = devices {
        for index in 0..devices.count() {
            let object = devices.objectAtIndex(index);
            let Ok(device) = object.downcast::<IOBluetoothDevice>() else {
                continue;
            };
            let Some(address) = (unsafe { device.addressString() }) else {
                continue;
            };
            let address = address.to_string();
            let address = normalize_address(&address).unwrap_or(address);
            let name = unsafe { device.name() }.to_string().trim().to_string();
            let name = if name.is_empty() { None } else { Some(name) };
            let channel = sdp_channel(&device);
            out.push(BluetoothDeviceInfo {
                address,
                name,
                channel,
            });
        }
    }
    unsafe { pool.drain() };
    Ok(out)
}
