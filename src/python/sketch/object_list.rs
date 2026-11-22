//! Python sequence adapter over the core constraint list.
//!
//! The storage lives in `crate::sketch::object_list`; this binding
//! class only adds the Python list protocol over the typed
//! `Py<PyConstraintBase>` items.

use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PySliceMethods;
use pyo3::types::{PyAny, PyList, PySlice};
use pyo3::PyClass;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::constraints::PyConstraintBase;
use crate::sketch::object_list::{shared_list, ObjectList, SharedList};

fn normalize_index(index: isize, len: usize) -> PyResult<usize> {
    let normalized = if index < 0 {
        len as isize + index
    } else {
        index
    };
    if normalized < 0 || normalized >= len as isize {
        return Err(PyIndexError::new_err("list index out of range"));
    }
    Ok(normalized as usize)
}

fn iter_to_vec<T>(value: &Bound<'_, PyAny>) -> PyResult<Vec<Py<T>>>
where
    T: PyClass,
{
    let mut items = Vec::new();
    for item in value.try_iter()? {
        items.push(item?.extract::<Py<T>>()?);
    }
    Ok(items)
}

/// A list-like collection of sketch constraints backed by the core store.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch", name = "ConstraintList")]
pub struct PyConstraintList {
    inner: SharedList<Py<PyConstraintBase>>,
}

impl PyConstraintList {
    /// Creates an empty collection.
    pub fn empty() -> Self {
        Self::from_shared(shared_list())
    }

    /// Creates an adapter over a document-owned collection.
    pub fn from_shared(inner: SharedList<Py<PyConstraintBase>>) -> Self {
        Self { inner }
    }

    /// Returns the shared collection.
    pub fn shared(&self) -> SharedList<Py<PyConstraintBase>> {
        self.inner.clone()
    }

    /// Creates a collection from an arbitrary Python iterable.
    pub fn from_iterable(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::from_vec(iter_to_vec::<PyConstraintBase>(value)?))
    }

    /// Creates a collection from pre-built Python objects.
    pub fn from_vec(items: Vec<Py<PyConstraintBase>>) -> Self {
        Self::from_shared(std::sync::Arc::new(std::sync::RwLock::new(
            ObjectList::from_vec(items),
        )))
    }

    fn read(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, ObjectList<Py<PyConstraintBase>>> {
        self.inner.read().unwrap()
    }

    fn write(
        &mut self,
    ) -> std::sync::RwLockWriteGuard<'_, ObjectList<Py<PyConstraintBase>>> {
        self.inner.write().unwrap()
    }

    /// Returns a snapshot of the contained objects.
    pub fn items(&self, py: Python<'_>) -> Vec<Py<PyConstraintBase>> {
        self.read()
            .items()
            .iter()
            .map(|item| item.clone_ref(py))
            .collect()
    }

    /// Appends an object.
    pub fn push(&mut self, item: Py<PyConstraintBase>) {
        self.write().push(item);
    }

    /// Replaces the contents.
    pub fn replace(&mut self, items: Vec<Py<PyConstraintBase>>) {
        self.write().replace(items);
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyConstraintList {
    #[new]
    #[pyo3(signature = (items = None))]
    fn new(
        _py: Python<'_>,
        items: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        match items {
            Some(value) => Self::from_iterable(value),
            None => Ok(Self::empty()),
        }
    }

    fn __len__(&self) -> usize {
        self.read().len()
    }

    fn __bool__(&self) -> bool {
        !self.read().is_empty()
    }

    fn __getitem__(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let guard = self.read();
        let items = guard.items();
        if let Ok(index) = key.extract::<isize>() {
            let index = normalize_index(index, items.len())?;
            return Ok(items[index].clone_ref(py).into_any());
        }
        if let Ok(slice) = key.cast::<PySlice>() {
            let indices = slice.indices(items.len() as isize)?;
            let selected: Vec<Py<PyConstraintBase>> = (0..indices.slicelength)
                .map(|i| {
                    let index = indices.start + i as isize * indices.step;
                    items[index as usize].clone_ref(py)
                })
                .collect();
            let result =
                PyList::new(py, selected.iter().map(|item| item.bind(py)))?;
            return Ok(result.into_any().unbind());
        }
        Err(PyTypeError::new_err(
            "list indices must be integers or slices",
        ))
    }

    fn __setitem__(
        &mut self,
        _py: Python<'_>,
        key: &Bound<'_, PyAny>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let len = self.read().len();
        if let Ok(index) = key.extract::<isize>() {
            let index = normalize_index(index, len)?;
            let item = value.extract::<Py<PyConstraintBase>>()?;
            self.write().items_mut()[index] = item;
            return Ok(());
        }
        if let Ok(slice) = key.cast::<PySlice>() {
            let indices = slice.indices(len as isize)?;
            let replacement = iter_to_vec::<PyConstraintBase>(value)?;
            if indices.step == 1 {
                let start = indices.start as usize;
                let end = start + indices.slicelength;
                self.write().items_mut().splice(start..end, replacement);
                return Ok(());
            }
            if replacement.len() != indices.slicelength {
                return Err(PyIndexError::new_err(
                    "attempt to assign sequence of size \
                     different from extended slice",
                ));
            }
            for (i, item) in replacement.into_iter().enumerate() {
                let index = indices.start + i as isize * indices.step;
                self.write().items_mut()[index as usize] = item;
            }
            return Ok(());
        }
        Err(PyTypeError::new_err(
            "list indices must be integers or slices",
        ))
    }

    fn __delitem__(&mut self, key: &Bound<'_, PyAny>) -> PyResult<()> {
        let len = self.read().len();
        if let Ok(index) = key.extract::<isize>() {
            let index = normalize_index(index, len)?;
            self.write().remove(index);
            return Ok(());
        }
        if let Ok(slice) = key.cast::<PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step == 1 {
                let start = indices.start as usize;
                let end = start + indices.slicelength;
                self.write().items_mut().drain(start..end);
                return Ok(());
            }
            let mut indexes: Vec<usize> = (0..indices.slicelength)
                .map(|i| (indices.start + i as isize * indices.step) as usize)
                .collect();
            indexes.sort_unstable();
            indexes.dedup();
            for index in indexes.into_iter().rev() {
                self.write().remove(index);
            }
            return Ok(());
        }
        Err(PyTypeError::new_err(
            "list indices must be integers or slices",
        ))
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let items = PyList::new(
            py,
            self.read().items().iter().map(|item| item.bind(py)),
        )?;
        items.call_method0("__iter__")
    }

    fn __contains__(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        for item in self.items(py) {
            if item.bind(py).eq(value)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn __eq__(
        &self,
        py: Python<'_>,
        other: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let items = PyList::new(
            py,
            self.read().items().iter().map(|item| item.bind(py)),
        )?;
        items.eq(other)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let items = PyList::new(
            py,
            self.read().items().iter().map(|item| item.bind(py)),
        )?;
        Ok(items.repr()?.to_string())
    }

    fn append(&mut self, value: Py<PyConstraintBase>) {
        self.write().push(value);
    }

    fn extend(
        &mut self,
        _py: Python<'_>,
        values: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let items = iter_to_vec::<PyConstraintBase>(values)?;
        self.write().extend(items);
        Ok(())
    }

    fn insert(&mut self, index: isize, value: Py<PyConstraintBase>) {
        let len = self.read().len() as isize;
        let index = if index < 0 {
            (len + index).max(0)
        } else {
            index.min(len)
        };
        self.write().insert(index as usize, value);
    }

    #[pyo3(signature = (index = -1))]
    fn pop(
        &mut self,
        py: Python<'_>,
        index: isize,
    ) -> PyResult<Py<PyConstraintBase>> {
        if self.read().is_empty() {
            return Err(PyIndexError::new_err("pop from empty list"));
        }
        let index = normalize_index(index, self.read().len())?;
        Ok(self.write().remove(index).clone_ref(py))
    }

    fn remove(
        &mut self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let index = {
            let items = self.items(py);
            let mut found = None;
            for (index, item) in items.iter().enumerate() {
                if item.bind(py).eq(value)? {
                    found = Some(index);
                    break;
                }
            }
            found
        };
        if let Some(index) = index {
            self.write().remove(index);
            return Ok(());
        }
        Err(PyValueError::new_err("list.remove(x): x not in list"))
    }

    fn clear(&mut self) {
        self.write().clear();
    }

    #[pyo3(signature = (value, start = 0, stop = None))]
    fn index(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        start: isize,
        stop: Option<isize>,
    ) -> PyResult<usize> {
        let items = self.items(py);
        let len = items.len() as isize;
        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        };
        let stop = stop
            .map(|stop| {
                if stop < 0 {
                    (len + stop).max(0)
                } else {
                    stop.min(len)
                }
            })
            .unwrap_or(len);
        let mut index = start;
        while index < stop {
            if items[index as usize].bind(py).eq(value)? {
                return Ok(index as usize);
            }
            index += 1;
        }
        Err(PyValueError::new_err("list.index(x): x not in list"))
    }

    fn count(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<usize> {
        let mut count = 0;
        for item in self.items(py) {
            if item.bind(py).eq(value)? {
                count += 1;
            }
        }
        Ok(count)
    }

    fn reverse(&mut self) {
        self.write().items_mut().reverse();
    }

    fn copy(&self, py: Python<'_>) -> Self {
        Self::from_vec(self.items(py))
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConstraintList>()?;
    Ok(())
}
