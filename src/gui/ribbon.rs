use super::{
    is_translated_point_in_box, panel::PanelEventData, EventSink, EventSource, Panel, PanelEvent,
};
use async_object::EventStream;
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
use windows::{
    core::HSTRING,
    Foundation::Numerics::{Vector2, Vector3},
    UI::Composition::{Compositor, ContainerVisual},
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
    panel: Box<dyn Panel>,
    container: ContainerVisual,
    limit: CellLimit,
}

impl Cell {
    fn new(panel: impl Panel + 'static, container: ContainerVisual, limit: CellLimit) -> Self {
        Self {
            panel: Box::new(panel),
            container,
            limit,
        }
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
        self.container.SetOffset(&Vector3 {
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
        self.panel.as_ref() as *const dyn Panel == other.panel.as_ref() as *const dyn Panel
    }
}

#[async_object_with_events_decl(pub Ribbon, pub WRibbon)]
pub struct RibbonImpl {
    compositor: Compositor,
    ribbon_container: ContainerVisual,
    orientation: RibbonOrientation,
    cells: Vec<Cell>,
    mouse_pos: Option<Vector2>,
}

impl RibbonImpl {
    fn new(
        compositor: Compositor,
        ribbon_container: ContainerVisual,
        orientation: RibbonOrientation,
    ) -> Self {
        Self {
            compositor,
            ribbon_container,
            orientation,
            cells: Vec::new(),
            mouse_pos: None,
        }
    }
}

#[async_object_impl(Ribbon, WRibbon)]
impl RibbonImpl {
    pub fn add_panel(
        &mut self,
        mut panel: impl Panel + 'static,
        limit: CellLimit,
    ) -> crate::Result<()> {
        let container = self.compositor.CreateContainerVisual()?;
        self.ribbon_container
            .Children()?
            .InsertAtTop(container.clone())?;
        panel.attach(container.clone())?;
        self.cells.push(Cell::new(panel, container, limit));
        self.resize_cells(self.ribbon_container.Size()?)?;
        Ok(())
    }
    pub fn orientation(&self) -> RibbonOrientation {
        self.orientation
    }
    pub fn cells(&self) -> Vec<Cell> {
        self.cells.clone()
    }
    fn set_mouse_pos(&mut self, mouse_pos: Vector2) {
        self.mouse_pos = Some(mouse_pos)
    }
    fn get_mouse_pos(&mut self) -> Option<Vector2> {
        self.mouse_pos
    }
    fn resize_cells(&mut self, size: Vector2) -> crate::Result<()> {
        self.ribbon_container.SetSize(size)?;
        if self.orientation == RibbonOrientation::Stack {
            for cell in &mut self.cells {
                let content_size = size.clone() * cell.limit.content_ratio.clone();
                let content_offset = Vector2 {
                    X: (size.X - content_size.X) / 2.,
                    Y: (size.Y - content_size.Y) / 2.,
                };
                cell.resize(content_offset, content_size)?;
            }
        } else {
            let limits = self.cells.iter().map(|c| c.limit).collect::<Vec<_>>();
            let hor = self.orientation == RibbonOrientation::Horizontal;
            let target = if hor { size.X } else { size.Y };
            let sizes = adjust_cells(limits, target);
            let mut pos: f32 = 0.;
            for i in 0..self.cells.len() {
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
                let cell = &mut self.cells[i];
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
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        container
            .Children()?
            .InsertAtTop(self.ribbon_container.clone())?;
        Ok(())
    }
    fn detach(&mut self) -> crate::Result<()> {
        if let Ok(parent) = self.ribbon_container.Parent() {
            parent.Children()?.Remove(&self.ribbon_container)?;
        }
        Ok(())
    }
}

impl Panel for Ribbon {
    fn id(&self) -> usize {
        self.id()
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.attach(container)
    }
    fn detach(&mut self) -> crate::Result<()> {
        self.detach()
    }
    fn clone_panel(&self) -> Box<dyn Panel> {
        Box::new(self.clone())
    }
}

impl EventSource<PanelEvent> for Ribbon {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Ribbon {
    async fn on_event(&mut self, event: PanelEvent) -> crate::Result<()> {
        match event.data {
            PanelEventData::Resized(size) => {
                self.translate_panel_event_resized(event.clone(), size)
                    .await
            }
            PanelEventData::MouseInput { state, button, .. } => {
                self.translate_slot_event_mouse_input(event.clone(), state, button)
                    .await
            }
            PanelEventData::CursorMoved(mouse_pos) => {
                self.translate_slot_event_cursor_moved(event.clone(), mouse_pos)
                    .await
            }
            _ => self.translate_panel_event_default(event.clone()).await,
        }?;
        self.send_event(event).await;
        Ok(())
    }
}

impl Ribbon {
    pub fn new(compositor: Compositor, orientation: RibbonOrientation) -> crate::Result<Self> {
        let ribbon_container = compositor.CreateContainerVisual()?;
        ribbon_container.SetComment(HSTRING::from("RIBBON_CONTAINER"))?;
        let ribbon_impl = RibbonImpl::new(compositor, ribbon_container, orientation);
        let ribbon = Self::create(ribbon_impl);
        Ok(ribbon)
    }

    async fn translate_panel_event_default(&mut self, event: PanelEvent) -> crate::Result<()> {
        // TODO: run simultaneosuly
        for mut cell in self.async_cells().await {
            cell.panel.on_event(event.clone()).await?;
        }
        Ok(())
    }

    async fn translate_panel_event_resized(
        &mut self,
        event: PanelEvent,
        size: Vector2,
    ) -> crate::Result<()> {
        self.async_resize_cells(size).await?;
        // TODO: run simultaneosuly
        for mut cell in self.cells() {
            let size = cell.container.Size()?;
            cell.panel
                .on_event(PanelEvent::new(
                    event.source.clone(),
                    PanelEventData::Resized(size),
                ))
                .await?;
        }
        Ok(())
    }

    async fn translate_slot_event_cursor_moved(
        &mut self,
        event: PanelEvent,
        mouse_pos: Vector2,
    ) -> crate::Result<()> {
        self.async_set_mouse_pos(mouse_pos).await;
        // TODO: run simultaneosuly
        for mut cell in self.async_cells().await {
            let mouse_pos = cell.translate_point(mouse_pos)?;
            cell.panel
                .on_event(PanelEvent::new(
                    event.source.clone(),
                    PanelEventData::CursorMoved(mouse_pos),
                ))
                .await?;
        }
        Ok(())
    }

    async fn translate_slot_event_mouse_input(
        &mut self,
        event: PanelEvent,
        state: ElementState,
        button: MouseButton,
    ) -> crate::Result<()> {
        if let Some(mouse_pos) = self.async_get_mouse_pos().await {
            // TODO: run simultaneosuly
            for mut cell in self.async_cells().await {
                let mouse_pos = cell.translate_point(mouse_pos)?;
                let in_slot = cell.is_translated_point_in_cell(mouse_pos)?;
                cell.panel
                    .on_event(PanelEvent::new(
                        event.source.clone(),
                        PanelEventData::MouseInput {
                            in_slot,
                            state,
                            button,
                        },
                    ))
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
