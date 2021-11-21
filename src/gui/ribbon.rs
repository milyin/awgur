use super::{
    is_translated_point_in_box, spawn_translate_window_events, FromVector2, IntoVector2, Slot,
    SlotPlug, SlotTag, TranslateWindowEvent,
};
use async_object::{Keeper, Tag};
use async_trait::async_trait;
use futures::task::Spawn;
use windows::{
    Foundation::Numerics::{Vector2, Vector3},
    UI::Composition::{Compositor, ContainerVisual},
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
};

#[derive(PartialEq, Clone, Copy)]
pub enum RibbonOrientation {
    Stack,
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Debug)]
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

struct Cell {
    slot: Slot,
    container: ContainerVisual,
    limit: CellLimit,
}

impl Cell {
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
        self.slot.resize(size)?;
        Ok(())
    }
}

pub struct RibbonImpl {
    compositor: Compositor,
    slot_plug: SlotPlug,
    container: ContainerVisual,
    orientation: RibbonOrientation,
    cells: Vec<Cell>,
    mouse_pos: Option<Vector2>,
}

impl RibbonImpl {
    fn new(
        compositor: &Compositor,
        slot: SlotTag,
        orientation: RibbonOrientation,
    ) -> crate::Result<Self> {
        let compositor = compositor.clone();
        let container = compositor.CreateContainerVisual()?;
        let slot_plug = slot.plug(container.clone().into())?;
        Ok(Self {
            compositor,
            slot_plug,
            container,
            orientation,
            cells: Vec::new(),
            mouse_pos: None,
        })
    }

    fn add_cell(&mut self, limit: CellLimit) -> crate::Result<SlotTag> {
        let container = self.compositor.CreateContainerVisual()?;
        let slot = Slot::new(
            container.clone(),
            format!(
                "{}/Ribbon_{}",
                self.slot_plug.tag().name(),
                self.cells.len() + 1
            ),
        )?;
        self.container.Children()?.InsertAtTop(container.clone())?;
        let tslot = slot.tag();
        self.cells.push(Cell {
            slot,
            container,
            limit,
        });
        self.resize_cells(self.container.Size()?)?;
        Ok(tslot)
    }

    fn resize_cells(&mut self, size: Vector2) -> crate::Result<()> {
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
    fn translate_window_event_default(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        for cell in &mut self.cells {
            cell.slot.send_window_event(event.clone())?;
        }
        Ok(())
    }

    fn translate_window_event_resized(&mut self, size: PhysicalSize<u32>) -> crate::Result<()> {
        let size = Vector2 {
            X: size.width as f32,
            Y: size.height as f32,
        };
        self.resize_cells(size)?;
        for cell in &mut self.cells {
            let size = cell.container.Size()?;
            cell.slot
                .send_window_event(WindowEvent::Resized((size.X as u32, size.Y as u32).into()))?;
        }
        Ok(())
    }

    fn translate_window_event_cursor_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        event: &WindowEvent<'static>,
    ) -> crate::Result<()> {
        let mouse_pos = position.into_vector2();
        self.mouse_pos = Some(mouse_pos);
        for cell in &mut self.cells {
            let mut event = event.clone();
            match event {
                WindowEvent::CursorMoved {
                    ref mut position, ..
                } => *position = cell.translate_point(mouse_pos)?.from_vector2(),
                _ => {}
            };
            cell.slot.send_window_event(event)?;
        }
        Ok(())
    }

    fn translate_window_event_mouse_input(
        &mut self,
        event: &WindowEvent<'static>,
    ) -> crate::Result<()> {
        if let Some(mouse_pos) = self.mouse_pos {
            for cell in &mut self.cells {
                let mouse_pos = cell.translate_point(mouse_pos)?;
                if cell.is_translated_point_in_cell(mouse_pos)? {
                    let event = event.clone();
                    cell.slot.send_window_event(event)?;
                }
            }
        }
        Ok(())
    }

    fn translate_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => self.translate_window_event_resized(size),
            ref event @ WindowEvent::CursorMoved { ref position, .. } => {
                self.translate_window_event_cursor_moved(*position, event)
            }
            ref event @ WindowEvent::MouseInput { .. } => {
                self.translate_window_event_mouse_input(event)
            }
            event => self.translate_window_event_default(event),
        }
    }
}

// fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
//     for cell in &mut self.cells {
//         let point = cell.translate_point(event.0)?;
//         cell.slot_keeper
//             .send_mouse_left_pressed(MouseLeftPressed(point))?
//     }
//     Ok(())
// }

// fn send_mouse_left_pressed_focused(
//     &mut self,
//     event: MouseLeftPressedFocused,
// ) -> crate::Result<()> {
//     for cell in &mut self.cells {
//         let point = cell.translate_point(event.0)?;
//         if cell.is_translated_point_in_cell(point)? {
//             return cell
//                 .slot_keeper
//                 .send_mouse_left_pressed_focused(MouseLeftPressedFocused(point));
//         }
//     }
//     Ok(())
// }

pub struct Ribbon(Keeper<RibbonImpl>);

impl Ribbon {
    pub fn new(
        spawner: impl Spawn,
        compositor: &Compositor,
        slot: SlotTag,
        orientation: RibbonOrientation,
    ) -> crate::Result<Self> {
        let ribbon = Self(Keeper::new(RibbonImpl::new(
            compositor,
            slot.clone(),
            orientation,
        )?));
        spawn_translate_window_events(spawner, slot, ribbon.tag())?;
        Ok(ribbon)
    }
    pub fn tag(&self) -> TRibbon {
        TRibbon(self.0.tag())
    }
    pub fn add_cell(&mut self, limit: CellLimit) -> crate::Result<SlotTag> {
        self.0.write(|v| v.add_cell(limit))
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

#[derive(Clone, PartialEq)]
pub struct TRibbon(Tag<RibbonImpl>);

impl TRibbon {
    pub async fn add_cell(&self, limit: CellLimit) -> crate::Result<Option<SlotTag>> {
        self.0
            .async_call_mut(|v| v.add_cell(limit))
            .await
            .transpose()
    }
}

#[async_trait]
impl TranslateWindowEvent for TRibbon {
    async fn translate_window_event(
        &self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>> {
        self.0
            .async_call_mut(|v| v.translate_window_event(event))
            .await
            .transpose()
    }
    async fn name(&self) -> String {
        self.0
            .async_call(|v| v.slot_plug.tag().name())
            .await
            .unwrap_or("(dropped)".into())
    }
}
