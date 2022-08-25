use super::{
    attach, is_translated_point_in_box, EventSink, EventSource, Panel, PanelEvent,
};
use async_event_streams::{EventBox, EventStream, EventStreams};
use async_std::sync::{Arc, RwLock};
use async_trait::async_trait;
use typed_builder::TypedBuilder;
use windows::{
    Foundation::Numerics::{Vector2, Vector3},
    UI::Composition::{Compositor, ContainerVisual, Visual},
};
use winit::event::{ElementState, MouseButton};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum RibbonOrientation {
    Stack,
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone)]
pub struct CellLimit {
    pub ratio: f32,
    pub min_size: f32,
    pub max_size: Option<f32>,
    pub content_ratio: Vector2,
}

impl CellLimit {
    pub fn new(
        ratio: f32,
        min_size: f32,
        max_size: Option<f32>,
        content_ratio: Option<Vector2>,
    ) -> Self {
        let content_ratio = content_ratio.unwrap_or(Vector2 { X: 1., Y: 1. });
        Self {
            ratio,
            min_size,
            max_size,
            content_ratio,
        }
    }

    pub fn set_size(&mut self, size: f32) {
        self.min_size = size;
        self.max_size = Some(size);
    }
}

impl Default for CellLimit {
    fn default() -> Self {
        Self {
            ratio: 1.,
            min_size: 0.,
            max_size: None,
            content_ratio: Vector2::new(1., 1.),
        }
    }
}

#[derive(Clone)]
pub struct Cell {
    panel: Arc<dyn Panel>,
    container: ContainerVisual,
    limit: CellLimit,
}

impl Cell {
    fn new(panel: Arc<dyn Panel>, compositor: &Compositor, limit: CellLimit) -> crate::Result<Self> {
        let panel = panel;
        let container = compositor.CreateContainerVisual()?;
        attach(&container, &*panel)?;
        Ok(Self {
            panel: panel.into(),
            container,
            limit,
        })
    }
    fn translate_point(&self, mut point: Vector2) -> crate::Result<Vector2> {
        let offset = self.container.Offset()?;
        point.X -= offset.X;
        point.Y -= offset.Y;
        Ok(point)
    }
    fn is_translated_point_in_cell(&self, point: Vector2) -> crate::Result<bool> {
        let size = self.container.Size()?;
        Ok(is_translated_point_in_box(point, size))
    }
    fn resize(&mut self, offset: Vector2, size: Vector2) -> crate::Result<()> {
        self.container.SetOffset(Vector3 {
            X: offset.X,
            Y: offset.Y,
            Z: 0.,
        })?;
        self.container.SetSize(size)?;
        Ok(())
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.panel.id() == other.panel.id()
    }
}

struct Core {
    orientation: RibbonOrientation,
    cells: Vec<Cell>,
    mouse_pos: Option<Vector2>,
}

impl Core {
    pub fn orientation(&self) -> RibbonOrientation {
        self.orientation
    }
    pub fn cells(&self) -> Vec<Cell> {
        self.cells.clone()
    }
    fn set_mouse_pos(&mut self, mouse_pos: Vector2) {
        self.mouse_pos = Some(mouse_pos)
    }
    fn get_mouse_pos(&self) -> Option<Vector2> {
        self.mouse_pos
    }
}

pub struct Ribbon {
    compositor: Compositor,
    ribbon_container: ContainerVisual,
    core: RwLock<Core>,
    panel_events: EventStreams<PanelEvent>,
    id: Arc<()>
}

#[derive(TypedBuilder)]
pub struct RibbonParams {
    compositor: Compositor,
    orientation: RibbonOrientation,
    #[builder(default)]
    cells: Vec<Cell>,
}

impl RibbonParams {
    pub fn add_panel(self, panel: Arc<dyn Panel>, limit: CellLimit) -> crate::Result<Self> {
        let mut this = self;
        this.cells.push(Cell::new(panel, &this.compositor, limit)?);
        Ok(this)
    }
}

impl TryFrom<RibbonParams> for Ribbon {
    type Error = crate::Error;

    fn try_from(value: RibbonParams) -> crate::Result<Self> {
        let ribbon_container = value.compositor.CreateContainerVisual()?;
        for cell in &value.cells {
            ribbon_container.Children()?.InsertAtTop(&cell.container)?;
        }
        // ribbon_container.SetComment(HSTRING::from("RIBBON_CONTAINER"))?;
        let core = RwLock::new(Core {
            orientation: value.orientation,
            cells: value.cells,
            mouse_pos: None,
        });
        Ok(Ribbon {
            compositor: value.compositor,
            ribbon_container,
            core,
            panel_events: EventStreams::new(),
            id: Arc::new(())
        })
    }
}

impl TryFrom<RibbonParams> for Arc<Ribbon> {
    type Error = crate::Error;

    fn try_from(value: RibbonParams) -> crate::Result<Self> {
        Ok(Arc::new(value.try_into()?))
    }
}

impl Ribbon {
    pub async fn add_panel(&self, panel: Arc<dyn Panel>, limit: CellLimit) -> crate::Result<()> {
        let cell = Cell::new(panel, &self.compositor, limit)?;
        self.ribbon_container
            .Children()?
            .InsertAtTop(&cell.container)?;
        self.core.write().await.cells.push(cell);
        self.resize_cells(self.ribbon_container.Size()?).await?;
        Ok(())
    }
    async fn resize_cells(&self, size: Vector2) -> crate::Result<()> {
        self.ribbon_container.SetSize(size)?;
        let (orientation, mut cells) = {
            let v = self.core.read().await;
            (v.orientation(), v.cells())
        };
        if orientation == RibbonOrientation::Stack {
            for cell in &mut cells {
                let content_size = size.clone() * cell.limit.content_ratio.clone();
                let content_offset = Vector2 {
                    X: (size.X - content_size.X) / 2.,
                    Y: (size.Y - content_size.Y) / 2.,
                };
                cell.resize(content_offset, content_size)?;
            }
        } else {
            let limits = cells.iter().map(|c| c.limit).collect::<Vec<_>>();
            let hor = orientation == RibbonOrientation::Horizontal;
            let target = if hor { size.X } else { size.Y };
            let sizes = adjust_cells(limits, target);
            let mut pos: f32 = 0.;
            for i in 0..cells.len() {
                let size = if hor {
                    Vector2 {
                        X: sizes[i],
                        Y: size.Y,
                    }
                } else {
                    Vector2 {
                        X: size.X,
                        Y: sizes[i],
                    }
                };
                let cell = &mut cells[i];
                let offset = if hor {
                    Vector2 { X: pos, Y: 0. }
                } else {
                    Vector2 { X: 0., Y: pos }
                };
                cell.resize(offset, size)?;
                pos += sizes[i];
            }
        }
        Ok(())
    }
}

impl Panel for Ribbon {
    fn outer_frame(&self) -> Visual {
        self.ribbon_container.clone().into()
    }
    fn id(&self) -> usize {
        Arc::as_ptr(&self.id) as usize
    }
}

impl EventSource<PanelEvent> for Ribbon {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Ribbon {
    async fn on_event(
        &self,
        event: &PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        match event {
            PanelEvent::Resized(size) => {
                self.translate_panel_event_resized(*size, source.clone())
                    .await
            }
            PanelEvent::MouseInput { state, button, .. } => {
                self.translate_slot_event_mouse_input(*state, *button, source.clone())
                    .await
            }
            PanelEvent::CursorMoved(mouse_pos) => {
                self.translate_slot_event_cursor_moved(*mouse_pos, source.clone())
                    .await
            }
            _ => {
                self.translate_panel_event_default(event, source.clone())
                    .await
            }
        }?;
        self.panel_events.send_event(event.clone(), source).await;
        Ok(())
    }
}

impl Ribbon {
    async fn translate_panel_event_default(
        &self,
        event: &PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        // TODO: run simultaneosuly
        let cells = self.core.read().await.cells();
        for cell in cells {
            cell.panel.on_event(event, source.clone()).await?;
        }
        Ok(())
    }

    async fn translate_panel_event_resized(
        &self,
        size: Vector2,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.resize_cells(size).await?;
        // TODO: run simultaneosuly
        let cells = self.core.read().await.cells();
        for cell in cells {
            let size = cell.container.Size()?;
            cell.panel
                .on_event(&PanelEvent::Resized(size), source.clone())
                .await?;
        }
        Ok(())
    }

    async fn translate_slot_event_cursor_moved(
        &self,
        mouse_pos: Vector2,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.core.write().await.set_mouse_pos(mouse_pos);
        // TODO: run simultaneosuly
        let cells = self.core.read().await.cells();
        for cell in cells {
            let mouse_pos = cell.translate_point(mouse_pos)?;
            cell.panel
                .on_event(&PanelEvent::CursorMoved(mouse_pos), source.clone())
                .await?;
        }
        Ok(())
    }

    async fn translate_slot_event_mouse_input(
        &self,
        state: ElementState,
        button: MouseButton,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let Some(mouse_pos) = self.core.read().await.get_mouse_pos() {
            // TODO: run simultaneosuly
            let cells = self.core.read().await.cells();
            for cell in cells {
                let mouse_pos = cell.translate_point(mouse_pos)?;
                let in_slot = cell.is_translated_point_in_cell(mouse_pos)?;
                cell.panel
                    .on_event(
                        &PanelEvent::MouseInput {
                            in_slot,
                            state,
                            button,
                        },
                        source.clone(),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

fn adjust_cells(limits: Vec<CellLimit>, mut target: f32) -> Vec<f32> {
    let mut lock = Vec::with_capacity(limits.len());
    let mut result = Vec::with_capacity(limits.len());
    lock.resize(limits.len(), false);
    result.resize(limits.len(), 0.);

    let mut sum_ratio = limits
        .iter()
        .map(|c| {
            assert!(c.ratio > 0.);
            c.ratio
        })
        .sum::<f32>();
    loop {
        let mut new_target = target;
        let mut all_lock = true;
        for i in 0..limits.len() {
            if !lock[i] {
                let mut share = target * limits[i].ratio / sum_ratio;
                if share <= limits[i].min_size {
                    share = limits[i].min_size;
                    lock[i] = true;
                }
                if let Some(max_size) = limits[i].max_size {
                    if share > max_size {
                        share = max_size;
                        lock[i] = true;
                    }
                }
                if lock[i] {
                    new_target -= share;
                    sum_ratio -= limits[i].ratio;
                    lock[i] = true;
                } else {
                    all_lock = false;
                }
                result[i] = share;
            }
        }
        if all_lock || new_target == target {
            break;
        }
        target = if new_target > 0. { new_target } else { 0. };
    }
    result
}
