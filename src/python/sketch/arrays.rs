//! Python bindings for sketch array definitions.

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3::types::PyDict;
use pyo3::PyClassInitializer;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::sketch::registry::{
    value_from_dict, value_to_py, PyEntityRegistry,
};
use crate::python::sketch::types::EntityId;
use crate::sketch::arrays::{
    ArrayDefinition as CoreDefinition, ArrayStrategy as CoreStrategy,
    CircularArray, CircularArrayStrategy, CurveAlongArray,
    CurveAlongArrayStrategy, InstancePlacement, PlacementKind,
};

type Members = Vec<(i64, Vec<EntityId>)>;
type Anchor = Option<((f64, f64), f64)>;
type MasterGeometry = (Vec<Py<PyAny>>, Vec<Py<PyAny>>, Vec<Py<PyAny>>);

/// Placement kind constants.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.arrays", name = "PlacementKind", frozen)]
pub struct PyPlacementKind;

#[gen_stub_pymethods]
#[pymethods]
impl PyPlacementKind {
    #[classattr]
    const ROTATION: &'static str = "rotation";

    #[classattr]
    const TRANSLATION: &'static str = "translation";

    #[classattr]
    const CURVE_ALIGNED: &'static str = "curve_aligned";
}

/// A rigid placement of one array instance.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "InstancePlacement",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyInstancePlacement {
    inner: InstancePlacement,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyInstancePlacement {
    #[new]
    #[pyo3(signature = (
        kind = "translation",
        angle = 0.0,
        center = (0.0, 0.0),
        delta = (0.0, 0.0),
        target_center = (0.0, 0.0)
    ))]
    fn new(
        kind: &str,
        angle: f64,
        center: (f64, f64),
        delta: (f64, f64),
        target_center: (f64, f64),
    ) -> PyResult<Self> {
        let kind = match kind {
            "rotation" => PlacementKind::Rotation,
            "translation" => PlacementKind::Translation,
            "curve_aligned" => PlacementKind::CurveAligned,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "unknown placement kind",
                ))
            }
        };
        Ok(PyInstancePlacement {
            inner: InstancePlacement {
                kind,
                angle,
                center,
                delta,
                target_center,
            },
        })
    }

    /// Placement kind: ``rotation``, ``translation`` or
    /// ``curve_aligned``.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind {
            PlacementKind::Rotation => "rotation",
            PlacementKind::Translation => "translation",
            PlacementKind::CurveAligned => "curve_aligned",
        }
    }

    #[getter]
    fn angle(&self) -> f64 {
        self.inner.angle
    }

    #[getter]
    fn center(&self) -> (f64, f64) {
        self.inner.center
    }

    #[getter]
    fn delta(&self) -> (f64, f64) {
        self.inner.delta
    }

    #[getter]
    fn target_center(&self) -> (f64, f64) {
        self.inner.target_center
    }

    /// Transforms an absolute point through the placement.
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        self.inner.transform_point(x, y)
    }

    /// Transforms a relative offset through the placement.
    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        self.inner.transform_offset(dx, dy)
    }
}

/// Shared base class for array strategies.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "ArrayStrategy",
    subclass,
    skip_from_py_object
)]
pub struct PyArrayStrategy {
    pub(crate) inner: Box<dyn CoreStrategy>,
}

impl PyArrayStrategy {
    fn from_box(inner: Box<dyn CoreStrategy>) -> Self {
        PyArrayStrategy { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArrayStrategy {
    #[getter]
    fn count(&self) -> u32 {
        self.inner.count()
    }

    #[setter]
    fn set_count(&mut self, value: u32) {
        self.inner.set_count(value);
    }

    #[getter]
    fn rotate_copies(&self) -> bool {
        self.inner.rotate_copies()
    }

    #[setter]
    fn set_rotate_copies(&mut self, value: bool) {
        self.inner.set_rotate_copies(value);
    }

    #[getter]
    fn needs_center_point(&self) -> bool {
        self.inner.needs_center_point()
    }

    #[getter]
    fn uses_template_anchor(&self) -> bool {
        self.inner.uses_template_anchor()
    }

    fn existing_master_id(&self) -> Option<i64> {
        self.inner.existing_master_id()
    }

    #[pyo3(signature = (template_center, registry = None))]
    fn template_placement(
        &self,
        template_center: (f64, f64),
        registry: Option<PyRef<'_, PyEntityRegistry>>,
    ) -> PyInstancePlacement {
        PyInstancePlacement {
            inner: self.inner.template_placement(
                template_center,
                registry.as_ref().map(|r| &r.inner),
            ),
        }
    }

    #[pyo3(signature = (template_center, registry = None))]
    fn member_placements(
        &self,
        template_center: (f64, f64),
        registry: Option<PyRef<'_, PyEntityRegistry>>,
    ) -> Vec<PyInstancePlacement> {
        self.inner
            .member_placements(
                template_center,
                registry.as_ref().map(|r| &r.inner),
            )
            .into_iter()
            .map(|inner| PyInstancePlacement { inner })
            .collect()
    }

    fn capture_master_frame(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
        array: PyRef<'_, PyArrayDefinition>,
    ) -> Option<((f64, f64), f64)> {
        self.inner
            .capture_master_frame(&registry.inner, &array.inner)
    }

    fn apply_frame(
        &self,
        mut registry: PyRefMut<'_, PyEntityRegistry>,
        array: PyRef<'_, PyArrayDefinition>,
        old_frame: ((f64, f64), f64),
    ) {
        self.inner
            .apply_frame(&mut registry.inner, &array.inner, old_frame);
    }

    fn create_master_geometry(
        &self,
        py: Python<'_>,
        center_pid: Option<EntityId>,
        radius_pt_pid: Option<EntityId>,
    ) -> PyResult<MasterGeometry> {
        let Some(strategy) =
            self.inner.as_any().downcast_ref::<CircularArrayStrategy>()
        else {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        };
        if strategy.radius <= 0.0 {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }
        let (Some(center_pid), Some(radius_pt_pid)) =
            (center_pid, radius_pt_pid)
        else {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        };
        let entities = py.import("raygeo.sketch.entities")?;
        let constraints = py.import("raygeo.sketch.constraints")?;
        let circle_temp_id = -(radius_pt_pid.0 as i64 + 1);
        let radius_point = entities.getattr("SketchPoint")?.call1((
            radius_pt_pid.0,
            strategy.center.0 + strategy.radius,
            strategy.center.1,
        ))?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("center_idx", center_pid.0)?;
        kwargs.set_item("radius_pt_idx", radius_pt_pid.0)?;
        kwargs.set_item("construction", true)?;
        let circle = entities
            .getattr("Circle")?
            .call((circle_temp_id,), Some(&kwargs))?;
        let radius_constraint = constraints.getattr("RadiusConstraint")?.call(
            (circle_temp_id,),
            Some(&{
                let kw = PyDict::new(py);
                kw.set_item("value", strategy.radius)?;
                kw
            }),
        )?;
        Ok((
            vec![radius_point.unbind()],
            vec![circle.unbind()],
            vec![radius_constraint.unbind()],
        ))
    }
}

/// Circular-array placement strategy.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CircularArrayStrategy",
    extends = PyArrayStrategy,
    skip_from_py_object
)]
pub struct PyCircularArrayStrategy;

fn circular(slf: &PyArrayStrategy) -> &CircularArrayStrategy {
    slf.inner
        .as_any()
        .downcast_ref::<CircularArrayStrategy>()
        .expect("circular array strategy")
}

fn circular_mut(slf: &mut PyArrayStrategy) -> &mut CircularArrayStrategy {
    slf.inner
        .as_any_mut()
        .downcast_mut::<CircularArrayStrategy>()
        .expect("circular array strategy")
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCircularArrayStrategy {
    #[new]
    #[pyo3(signature = (
        count = 6,
        rotate_copies = true,
        total_angle_deg = 360.0,
        center = (0.0, 0.0),
        radius = 0.0
    ))]
    fn new(
        count: u32,
        rotate_copies: bool,
        total_angle_deg: f64,
        center: (f64, f64),
        radius: f64,
    ) -> PyClassInitializer<Self> {
        let strategy = CircularArrayStrategy {
            count,
            rotate_copies,
            total_angle_deg,
            center,
            radius,
        };
        PyClassInitializer::from(PyArrayStrategy::from_box(Box::new(strategy)))
            .add_subclass(PyCircularArrayStrategy)
    }

    #[getter]
    fn total_angle_deg(slf: PyRef<'_, Self>) -> f64 {
        circular(&slf.into_super()).total_angle_deg
    }

    #[setter]
    fn set_total_angle_deg(slf: PyRefMut<'_, Self>, value: f64) {
        circular_mut(&mut slf.into_super()).total_angle_deg = value;
    }

    #[getter]
    fn center(slf: PyRef<'_, Self>) -> (f64, f64) {
        circular(&slf.into_super()).center
    }

    #[setter]
    fn set_center(slf: PyRefMut<'_, Self>, value: (f64, f64)) {
        circular_mut(&mut slf.into_super()).center = value;
    }

    #[getter]
    fn radius(slf: PyRef<'_, Self>) -> f64 {
        circular(&slf.into_super()).radius
    }

    #[setter]
    fn set_radius(slf: PyRefMut<'_, Self>, value: f64) {
        circular_mut(&mut slf.into_super()).radius = value;
    }
}

/// Curve-along placement strategy.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CurveAlongArrayStrategy",
    extends = PyArrayStrategy,
    skip_from_py_object
)]
pub struct PyCurveAlongArrayStrategy;

fn curve(slf: &PyArrayStrategy) -> &CurveAlongArrayStrategy {
    slf.inner
        .as_any()
        .downcast_ref::<CurveAlongArrayStrategy>()
        .expect("curve-along array strategy")
}

fn curve_mut(slf: &mut PyArrayStrategy) -> &mut CurveAlongArrayStrategy {
    slf.inner
        .as_any_mut()
        .downcast_mut::<CurveAlongArrayStrategy>()
        .expect("curve-along array strategy")
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCurveAlongArrayStrategy {
    #[new]
    #[pyo3(signature = (
        count = 6,
        rotate_copies = true,
        path_entity_id = -1,
        align_to_tangent = true,
        offset_to_start = 0.0,
        spacing = 0.0
    ))]
    fn new(
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
    ) -> PyClassInitializer<Self> {
        let strategy = CurveAlongArrayStrategy {
            count,
            rotate_copies,
            path_entity_id,
            align_to_tangent,
            offset_to_start,
            spacing,
        };
        PyClassInitializer::from(PyArrayStrategy::from_box(Box::new(strategy)))
            .add_subclass(PyCurveAlongArrayStrategy)
    }

    #[getter]
    fn path_entity_id(slf: PyRef<'_, Self>) -> i64 {
        curve(&slf.into_super()).path_entity_id
    }

    #[setter]
    fn set_path_entity_id(slf: PyRefMut<'_, Self>, value: i64) {
        curve_mut(&mut slf.into_super()).path_entity_id = value;
    }

    #[getter]
    fn align_to_tangent(slf: PyRef<'_, Self>) -> bool {
        curve(&slf.into_super()).align_to_tangent
    }

    #[setter]
    fn set_align_to_tangent(slf: PyRefMut<'_, Self>, value: bool) {
        curve_mut(&mut slf.into_super()).align_to_tangent = value;
    }

    #[getter]
    fn offset_to_start(slf: PyRef<'_, Self>) -> f64 {
        curve(&slf.into_super()).offset_to_start
    }

    #[setter]
    fn set_offset_to_start(slf: PyRefMut<'_, Self>, value: f64) {
        curve_mut(&mut slf.into_super()).offset_to_start = value;
    }

    #[getter]
    fn spacing(slf: PyRef<'_, Self>) -> f64 {
        curve(&slf.into_super()).spacing
    }

    #[setter]
    fn set_spacing(slf: PyRefMut<'_, Self>, value: f64) {
        curve_mut(&mut slf.into_super()).spacing = value;
    }
}

/// Persistent definition of a sketch array.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "Array",
    subclass,
    skip_from_py_object
)]
pub struct PyArrayDefinition {
    pub(crate) inner: CoreDefinition,
    pub(crate) cached_guide_sig: Option<Py<PyAny>>,
    pub(crate) cached_template_sig: Option<Py<PyAny>>,
    pub(crate) cached_guide_frame: Option<((f64, f64), f64)>,
}

impl PyArrayDefinition {
    pub(crate) fn wrap(inner: CoreDefinition) -> Self {
        PyArrayDefinition {
            inner,
            cached_guide_sig: None,
            cached_template_sig: None,
            cached_guide_frame: None,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArrayDefinition {
    #[staticmethod]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        total_angle_deg = 360.0,
        rotate_copies = true
    ))]
    fn circular(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        total_angle_deg: f64,
        rotate_copies: bool,
    ) -> Self {
        PyArrayDefinition::wrap(CoreDefinition::Circular(CircularArray::new(
            uid,
            guide_circle_id.0,
            members
                .unwrap_or_default()
                .into_iter()
                .map(|(slot, eids)| {
                    (slot, eids.into_iter().map(|id| id.0).collect())
                })
                .collect(),
            count,
            total_angle_deg,
            rotate_copies,
        )))
    }

    #[staticmethod]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        rotate_copies = true,
        path_entity_id = -1,
        align_to_tangent = true,
        offset_to_start = 0.0,
        spacing = 0.0,
        template_anchor = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn curve_along(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
        template_anchor: Anchor,
    ) -> Self {
        PyArrayDefinition::wrap(CoreDefinition::CurveAlong(
            CurveAlongArray::new(
                uid,
                guide_circle_id.0,
                members
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(slot, eids)| {
                        (slot, eids.into_iter().map(|id| id.0).collect())
                    })
                    .collect(),
                count,
                rotate_copies,
                path_entity_id,
                align_to_tangent,
                offset_to_start,
                spacing,
                template_anchor,
            ),
        ))
    }

    /// Array mode: ``circular`` or ``curve_along``.
    #[getter]
    fn mode(&self) -> &'static str {
        self.inner.mode().as_str()
    }

    /// Unique array id.
    #[getter]
    fn uid(&self) -> String {
        self.inner.common().uid.clone()
    }

    #[setter]
    fn set_uid(&mut self, value: String) {
        self.inner.common_mut().uid = value;
    }

    /// Guide circle or guide path entity id.
    #[getter]
    fn guide_circle_id(&self) -> EntityId {
        EntityId(self.inner.common().guide_circle_id)
    }

    #[setter]
    fn set_guide_circle_id(&mut self, value: EntityId) {
        self.inner.common_mut().guide_circle_id = value.0;
    }

    /// Array members as ``(slot, [entity_id, ...])`` pairs.
    #[getter]
    fn members(&self) -> Vec<(i64, Vec<EntityId>)> {
        self.inner
            .common()
            .members
            .iter()
            .map(|(slot, eids)| {
                (*slot, eids.iter().copied().map(EntityId).collect())
            })
            .collect()
    }

    #[setter]
    fn set_members(&mut self, value: Members) {
        self.inner.common_mut().members = value
            .into_iter()
            .map(|(slot, eids)| {
                (slot, eids.into_iter().map(|id| id.0).collect())
            })
            .collect();
    }

    /// Standalone member points as ``{slot: [point_id, ...]}``.
    #[getter]
    fn standalone_pids<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (slot, pids) in &self.inner.common().standalone_pids {
            dict.set_item(
                *slot,
                pids.iter().copied().map(EntityId).collect::<Vec<_>>(),
            )?;
        }
        Ok(dict)
    }

    #[setter]
    fn set_standalone_pids(
        &mut self,
        value: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let mut pairs = Vec::new();
        for (key, pids) in value.iter() {
            let slot: i64 = key.extract()?;
            let ids: Vec<EntityId> = pids.extract()?;
            pairs.push((slot, ids.into_iter().map(|id| id.0).collect()));
        }
        self.inner.common_mut().standalone_pids = pairs;
        Ok(())
    }

    /// Number of members.
    #[getter]
    fn count(&self) -> u32 {
        self.inner.common().count
    }

    #[setter]
    fn set_count(&mut self, value: u32) {
        self.inner.common_mut().count = value;
    }

    /// Whether copies are rotated with the guide.
    #[getter]
    fn rotate_copies(&self) -> bool {
        self.inner.common().rotate_copies
    }

    #[setter]
    fn set_rotate_copies(&mut self, value: bool) {
        self.inner.common_mut().rotate_copies = value;
    }

    /// Circular-array sweep angle.
    #[getter]
    fn total_angle_deg(&self) -> f64 {
        match &self.inner {
            CoreDefinition::Circular(a) => a.total_angle_deg,
            CoreDefinition::CurveAlong(_) => 360.0,
        }
    }

    #[setter]
    fn set_total_angle_deg(&mut self, value: f64) {
        if let CoreDefinition::Circular(a) = &mut self.inner {
            a.total_angle_deg = value;
        }
    }

    /// Curve-along guide path entity id.
    #[getter]
    fn path_entity_id(&self) -> i64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.path_entity_id,
            CoreDefinition::Circular(_) => -1,
        }
    }

    #[setter]
    fn set_path_entity_id(&mut self, value: i64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.path_entity_id = value;
        }
    }

    /// Whether curve-along copies follow the tangent.
    #[getter]
    fn align_to_tangent(&self) -> bool {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.align_to_tangent,
            CoreDefinition::Circular(_) => true,
        }
    }

    #[setter]
    fn set_align_to_tangent(&mut self, value: bool) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.align_to_tangent = value;
        }
    }

    /// Curve-along start offset.
    #[getter]
    fn offset_to_start(&self) -> f64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.offset_to_start,
            CoreDefinition::Circular(_) => 0.0,
        }
    }

    #[setter]
    fn set_offset_to_start(&mut self, value: f64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.offset_to_start = value;
        }
    }

    /// Curve-along spacing.
    #[getter]
    fn spacing(&self) -> f64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.spacing,
            CoreDefinition::Circular(_) => 0.0,
        }
    }

    #[setter]
    fn set_spacing(&mut self, value: f64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.spacing = value;
        }
    }

    /// Curve-along template anchor.
    #[getter]
    fn template_anchor(&self) -> Anchor {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.template_anchor,
            CoreDefinition::Circular(_) => None,
        }
    }

    #[setter]
    fn set_template_anchor(&mut self, value: Anchor) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.template_anchor = value;
        }
    }

    /// Living members, pruned to surviving entities.
    fn living_members(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> Vec<(i64, Vec<EntityId>)> {
        self.inner
            .living_members(&registry.inner)
            .into_iter()
            .map(|(slot, eids)| {
                (slot, eids.into_iter().map(EntityId).collect())
            })
            .collect()
    }

    /// Flat list of surviving member entity ids.
    fn living_entity_ids(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> Vec<EntityId> {
        self.inner
            .living_entity_ids(&registry.inner)
            .into_iter()
            .map(EntityId)
            .collect()
    }

    /// Slot numbers of surviving members.
    fn occupied_slots(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> Vec<i64> {
        let mut slots: Vec<i64> = self
            .inner
            .occupied_slots(&registry.inner)
            .into_iter()
            .collect();
        slots.sort_unstable();
        slots
    }

    /// Quantized guide geometry signature.
    fn guide_signature<'py>(
        &self,
        py: Python<'py>,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> PyResult<Bound<'py, PyAny>> {
        value_to_py(py, &self.inner.guide_signature(&registry.inner))
    }

    /// Quantized template geometry signature.
    fn template_signature<'py>(
        &self,
        py: Python<'py>,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> PyResult<Bound<'py, PyAny>> {
        value_to_py(py, &self.inner.template_signature(&registry.inner))
    }

    /// Whether the point is this array's guide radius point.
    fn is_guide_radius_point(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
        pid: EntityId,
    ) -> bool {
        self.inner.is_guide_radius_point(&registry.inner, pid.0)
    }

    /// Drops deleted entities from member groups.
    fn prune(&mut self, registry: PyRef<'_, PyEntityRegistry>) -> bool {
        self.inner.prune(&registry.inner)
    }

    /// Guide entity id (circle or path).
    #[getter]
    fn guide_entity_id(&self) -> EntityId {
        EntityId(self.inner.common().guide_circle_id)
    }

    #[getter]
    fn _cached_guide_sig(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.cached_guide_sig.as_ref().map(|v| v.clone_ref(py))
    }

    #[setter]
    #[allow(non_snake_case)]
    fn set__cached_guide_sig(&mut self, value: Option<Py<PyAny>>) {
        self.cached_guide_sig = value;
    }

    #[getter]
    fn _cached_template_sig(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.cached_template_sig.as_ref().map(|v| v.clone_ref(py))
    }

    #[setter]
    #[allow(non_snake_case)]
    fn set__cached_template_sig(&mut self, value: Option<Py<PyAny>>) {
        self.cached_template_sig = value;
    }

    #[getter]
    fn _cached_guide_frame(&self) -> Option<((f64, f64), f64)> {
        self.cached_guide_frame
    }

    #[setter]
    #[allow(non_snake_case)]
    fn set__cached_guide_frame(&mut self, value: Option<((f64, f64), f64)>) {
        self.cached_guide_frame = value;
    }

    /// Builds the strategy for this definition.
    fn make_strategy(
        &self,
        py: Python<'_>,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> PyResult<Py<PyAny>> {
        match &self.inner {
            CoreDefinition::Circular(_) => {
                let inner = self
                    .inner
                    .make_circular_strategy(&registry.inner)
                    .ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err(
                            "guide circle is missing",
                        )
                    })?;
                let base = PyArrayStrategy::from_box(Box::new(inner));
                Ok(Py::new(
                    py,
                    PyClassInitializer::from(base)
                        .add_subclass(PyCircularArrayStrategy),
                )?
                .into_any())
            }
            CoreDefinition::CurveAlong(_) => {
                let inner =
                    self.inner.make_curve_strategy().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err(
                            "invalid curve-along strategy",
                        )
                    })?;
                let base = PyArrayStrategy::from_box(Box::new(inner));
                Ok(Py::new(
                    py,
                    PyClassInitializer::from(base)
                        .add_subclass(PyCurveAlongArrayStrategy),
                )?
                .into_any())
            }
        }
    }

    /// Writes a strategy's parameters onto the definition.
    fn commit(&mut self, strategy: &Bound<'_, PyAny>) -> PyResult<()> {
        let Ok(base) = strategy.extract::<PyRef<PyArrayStrategy>>() else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "unsupported array strategy",
            ));
        };
        if let Some(s) =
            base.inner.as_any().downcast_ref::<CircularArrayStrategy>()
        {
            self.inner.commit_circular(s);
        } else if let Some(s) = base
            .inner
            .as_any()
            .downcast_ref::<CurveAlongArrayStrategy>()
        {
            self.inner.commit_curve(s);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "unsupported array strategy",
            ));
        }
        Ok(())
    }

    /// Whether a strategy requires full re-distribution.
    fn params_changed(&self, strategy: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(base) = strategy.extract::<PyRef<PyArrayStrategy>>() else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "unsupported array strategy",
            ));
        };
        if let Some(s) =
            base.inner.as_any().downcast_ref::<CircularArrayStrategy>()
        {
            Ok(self.inner.params_changed_circular(s))
        } else if let Some(s) = base
            .inner
            .as_any()
            .downcast_ref::<CurveAlongArrayStrategy>()
        {
            Ok(self.inner.params_changed_curve(s))
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "unsupported array strategy",
            ))
        }
    }

    /// Recomputes the sync caches from live geometry.
    fn refresh_caches(
        &mut self,
        py: Python<'_>,
        registry: PyRef<'_, PyEntityRegistry>,
        strategy: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        if let Ok(base) = strategy.extract::<PyRef<PyArrayStrategy>>() {
            self.cached_guide_frame = if let Some(s) =
                base.inner.as_any().downcast_ref::<CircularArrayStrategy>()
            {
                CircularArrayStrategy::capture_master_frame(
                    s,
                    &registry.inner,
                    self.inner.common().guide_circle_id,
                )
            } else {
                None
            };
        } else {
            self.cached_guide_frame = None;
        }
        self.cached_guide_sig = Some(
            value_to_py(py, &self.inner.guide_signature(&registry.inner))?
                .unbind(),
        );
        self.cached_template_sig = Some(
            value_to_py(py, &self.inner.template_signature(&registry.inner))?
                .unbind(),
        );
        Ok(())
    }

    /// Re-positions the template onto guide position 0.
    fn reanchor_template(
        &mut self,
        strategy: &Bound<'_, PyAny>,
        mut registry: PyRefMut<'_, PyEntityRegistry>,
        template_eids: Vec<EntityId>,
        standalone_pids: Vec<EntityId>,
    ) -> PyResult<()> {
        let Ok(base) = strategy.extract::<PyRef<PyArrayStrategy>>() else {
            return Ok(());
        };
        let Some(s) = base
            .inner
            .as_any()
            .downcast_ref::<CurveAlongArrayStrategy>()
        else {
            return Ok(());
        };
        let template: Vec<u64> =
            template_eids.into_iter().map(|id| id.0).collect();
        let standalone: Vec<u64> =
            standalone_pids.into_iter().map(|id| id.0).collect();
        s.reanchor_template(
            &mut registry.inner,
            &mut self.inner,
            &template,
            &standalone,
        );
        Ok(())
    }

    /// Creates an array from a strategy.
    #[staticmethod]
    fn from_strategy(
        py: Python<'_>,
        strategy: &Bound<'_, PyAny>,
        uid: String,
        guide_circle_id: EntityId,
        members: Members,
        count: u32,
        template_anchor: Anchor,
    ) -> PyResult<Py<PyAny>> {
        let members = members
            .into_iter()
            .map(|(slot, eids)| {
                (slot, eids.into_iter().map(|id| id.0).collect())
            })
            .collect();
        let Ok(base) = strategy.extract::<PyRef<PyArrayStrategy>>() else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "unsupported array strategy",
            ));
        };
        if let Some(s) =
            base.inner.as_any().downcast_ref::<CircularArrayStrategy>()
        {
            let inner = CoreDefinition::Circular(CircularArray::new(
                uid,
                guide_circle_id.0,
                members,
                count,
                s.total_angle_deg,
                s.rotate_copies,
            ));
            return Ok(Py::new(
                py,
                (PyCircularArray, PyArrayDefinition::wrap(inner)),
            )?
            .into_any());
        }
        if let Some(s) = base
            .inner
            .as_any()
            .downcast_ref::<CurveAlongArrayStrategy>()
        {
            let inner = CoreDefinition::CurveAlong(CurveAlongArray::new(
                uid,
                guide_circle_id.0,
                members,
                count,
                s.rotate_copies,
                s.path_entity_id,
                s.align_to_tangent,
                s.offset_to_start,
                s.spacing,
                template_anchor,
            ));
            return Ok(Py::new(
                py,
                (PyCurveAlongArray, PyArrayDefinition::wrap(inner)),
            )?
            .into_any());
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "unsupported array strategy",
        ))
    }

    /// Creates a copy of the definition.
    fn copy(&self) -> PyArrayDefinition {
        PyArrayDefinition::wrap(self.inner.clone())
    }

    /// Serializes the definition.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        let dict = PyDict::new(py);
        if let serde_json::Value::Object(map) = value {
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, &item)?)?;
            }
        }
        Ok(dict)
    }

    /// Deserializes a definition.
    #[staticmethod]
    fn from_dict(
        py: Python<'_>,
        dict: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let value = value_from_dict(dict)?;
        let inner = CoreDefinition::from_json(&value).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(
                "unsupported sketch array definition",
            )
        })?;
        Ok(match &inner {
            CoreDefinition::Circular(_) => {
                Py::new(py, (PyCircularArray, PyArrayDefinition::wrap(inner)))?
                    .into_any()
            }
            CoreDefinition::CurveAlong(_) => Py::new(
                py,
                (PyCurveAlongArray, PyArrayDefinition::wrap(inner)),
            )?
            .into_any(),
        })
    }

    /// Stores the sync signatures after a cycle.
    fn update_caches(
        &mut self,
        py: Python<'_>,
        guide_sig: &Bound<'_, PyAny>,
        template_sig: &Bound<'_, PyAny>,
    ) {
        self.cached_guide_sig = Some(guide_sig.clone().unbind());
        self.cached_template_sig = Some(template_sig.clone().unbind());
        let _ = py;
    }

    /// True if stored sync caches disagree with the given signatures.
    fn signatures_changed(
        &self,
        py: Python<'_>,
        guide_sig: &Bound<'_, PyAny>,
        template_sig: &Bound<'_, PyAny>,
    ) -> bool {
        let guide_changed = self
            .cached_guide_sig
            .as_ref()
            .map(|cached| !cached.bind(py).eq(guide_sig).unwrap_or(false))
            .unwrap_or(false);
        let template_changed = self
            .cached_template_sig
            .as_ref()
            .map(|cached| !cached.bind(py).eq(template_sig).unwrap_or(false))
            .unwrap_or(false);
        guide_changed || template_changed
    }

    /// Captures editable state for undo.
    fn snapshot<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        value_to_py(py, &self.inner.snapshot())
    }

    /// Restores editable state from a snapshot.
    fn restore(&mut self, state: &Bound<'_, PyAny>) -> PyResult<()> {
        let dict = state.cast::<PyDict>()?;
        let value = value_from_dict(dict)?;
        self.inner.restore(&value);
        Ok(())
    }

    fn __copy__(&self) -> PyArrayDefinition {
        self.copy()
    }
}

/// Concrete circular array.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CircularArray",
    extends = PyArrayDefinition
)]
pub struct PyCircularArray;

#[gen_stub_pymethods]
#[pymethods]
impl PyCircularArray {
    #[new]
    #[allow(deprecated)]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        total_angle_deg = 360.0,
        rotate_copies = true
    ))]
    fn new(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        total_angle_deg: f64,
        rotate_copies: bool,
    ) -> PyClassInitializer<Self> {
        let inner = CoreDefinition::Circular(CircularArray::new(
            uid,
            guide_circle_id.0,
            members
                .unwrap_or_default()
                .into_iter()
                .map(|(slot, eids)| {
                    (slot, eids.into_iter().map(|id| id.0).collect())
                })
                .collect(),
            count,
            total_angle_deg,
            rotate_copies,
        ));
        PyClassInitializer::from(PyArrayDefinition::wrap(inner))
            .add_subclass(PyCircularArray)
    }
}

/// Concrete curve-along array.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CurveAlongArray",
    extends = PyArrayDefinition
)]
pub struct PyCurveAlongArray;

#[gen_stub_pymethods]
#[pymethods]
impl PyCurveAlongArray {
    #[new]
    #[allow(deprecated)]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        rotate_copies = true,
        path_entity_id = -1,
        align_to_tangent = true,
        offset_to_start = 0.0,
        spacing = 0.0,
        template_anchor = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
        template_anchor: Anchor,
    ) -> PyClassInitializer<Self> {
        let inner = CoreDefinition::CurveAlong(CurveAlongArray::new(
            uid,
            guide_circle_id.0,
            members
                .unwrap_or_default()
                .into_iter()
                .map(|(slot, eids)| {
                    (slot, eids.into_iter().map(|id| id.0).collect())
                })
                .collect(),
            count,
            rotate_copies,
            path_entity_id,
            align_to_tangent,
            offset_to_start,
            spacing,
            template_anchor,
        ));
        PyClassInitializer::from(PyArrayDefinition::wrap(inner))
            .add_subclass(PyCurveAlongArray)
    }
}

/// Returns the logical center of the template entities.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction]
fn resolve_template_center(
    registry: PyRef<'_, PyEntityRegistry>,
    template_entity_ids: Vec<EntityId>,
    _template_points: &Bound<'_, PyAny>,
) -> (f64, f64) {
    let ids = template_entity_ids.into_iter().map(|id| id.0).collect();
    crate::sketch::entity_group::EntityGroup::new(ids).center(&registry.inner)
}

/// Returns the array whose master entity is the given one.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction]
fn find_array_for_entity(
    py: Python<'_>,
    arrays: &Bound<'_, PyAny>,
    entity_id: EntityId,
) -> PyResult<Option<Py<PyArrayDefinition>>> {
    for item in arrays.try_iter()? {
        let array = item?.extract::<Py<PyArrayDefinition>>()?;
        if array.borrow(py).inner.common().guide_circle_id == entity_id.0 {
            return Ok(Some(array));
        }
    }
    Ok(None)
}

/// Cumulative arc lengths of a polyline.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction(name = "_cumulative_lengths")]
fn cumulative_lengths(polyline: Vec<(f64, f64)>) -> Vec<f64> {
    crate::sketch::arrays::cumulative_lengths(&polyline)
}

/// Point and unit tangent at an arc length.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction(name = "_point_at_arclength")]
fn point_at_arclength(
    polyline: Vec<(f64, f64)>,
    cum: Vec<f64>,
    s: f64,
) -> ((f64, f64), (f64, f64)) {
    crate::sketch::arrays::point_at_arclength(&polyline, &cum, s)
}

/// Samples a guide path at equal arc-length intervals.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction]
#[pyo3(signature = (registry, path_entity_id, count, offset_to_start = 0.0))]
fn sample_path(
    registry: PyRef<'_, PyEntityRegistry>,
    path_entity_id: EntityId,
    count: u32,
    offset_to_start: f64,
) -> Vec<((f64, f64), f64)> {
    crate::sketch::arrays::sample_path(
        &registry.inner,
        path_entity_id.0,
        count,
        offset_to_start,
    )
}

/// Total arc length of a guide path.
#[gen_stub_pyfunction(module = "raygeo.sketch.arrays")]
#[pyfunction]
fn path_length(
    registry: PyRef<'_, PyEntityRegistry>,
    path_entity_id: EntityId,
) -> f64 {
    crate::sketch::arrays::path_length(&registry.inner, path_entity_id.0)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let arrays_mod = PyModule::new(py, "arrays")?;

    arrays_mod
        .setattr("__doc__", "Persistent definitions of sketch arrays.")?;
    arrays_mod.add_class::<PyArrayDefinition>()?;
    arrays_mod.add_class::<PyCircularArray>()?;
    arrays_mod.add_class::<PyCurveAlongArray>()?;
    arrays_mod.add("ArrayDefinition", arrays_mod.getattr("Array")?)?;
    arrays_mod
        .add_function(wrap_pyfunction!(cumulative_lengths, &arrays_mod)?)?;
    arrays_mod
        .add_function(wrap_pyfunction!(point_at_arclength, &arrays_mod)?)?;
    arrays_mod.add_function(wrap_pyfunction!(
        resolve_template_center,
        &arrays_mod
    )?)?;
    arrays_mod
        .add_function(wrap_pyfunction!(find_array_for_entity, &arrays_mod)?)?;
    arrays_mod.add_function(wrap_pyfunction!(sample_path, &arrays_mod)?)?;
    arrays_mod.add_function(wrap_pyfunction!(path_length, &arrays_mod)?)?;
    arrays_mod.add_class::<PyPlacementKind>()?;
    arrays_mod.add_class::<PyInstancePlacement>()?;
    arrays_mod.add_class::<PyArrayStrategy>()?;
    arrays_mod.add_class::<PyCircularArrayStrategy>()?;
    arrays_mod.add_class::<PyCurveAlongArrayStrategy>()?;

    m.add_submodule(&arrays_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.arrays", &arrays_mod)?;

    Ok(())
}
