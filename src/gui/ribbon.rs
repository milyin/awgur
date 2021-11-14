use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use async_object::{Keeper, Tag};
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{
    Foundation::Numerics::{Vector2, Vector3},
    UI::Composition::{Compositor, ContainerVisual},
};

use super::{SlotKeeper, SlotPlug, SlotTag};
use crate::{
    event::{MouseLeftPressed, MouseLeftPressedFocused, ReceiveSlotEvent, SendSlotEvent, SlotSize},
    unwrap_err,
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
    pub content_ratio: Vector2,
    pub min_size: f32,
    pub max_size: Option<f32>,
}

impl CellLimit {
    pub fn new(ratio: f32, content_ratio: Vector2, min_size: f32, max_size: Option<f32>) -> Self {
        Self {
            ratio,
            content_ratio,
            min_size,
            max_size,
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
            content_ratio: Vector2::new(1., 1.),
            min_size: 0.,
            max_size: None,
        }
    }
}

struct Cell {
    slot_keeper: SlotKeeper,
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
        Ok(point.X >= 0. && point.X < size.X && point.Y >= 0. && point.Y < size.Y)
    }
}

pub struct Ribbon {
    compositor: Compositor,
    slot: SlotPlug,
    container: ContainerVisual,
    orientation: RibbonOrientation,
    cells: Vec<Cell>,
}

impl Ribbon {
    pub fn new(
        compositor: Compositor,
        slot: SlotTag,
        orientation: RibbonOrientation,
    ) -> crate::Result<Self> {
        let container = compositor.CreateContainerVisual()?;
        let slot = slot.plug(container.clone().into())?;
        Ok(Self {
            compositor,
            slot,
            container,
            orientation,
            cells: Vec::new(),
        })
    }

    pub fn add_cell(&mut self, limit: CellLimit) -> crate::Result<SlotTag> {
        let container = self.compositor.CreateContainerVisual()?;
        let slot_keeper = SlotKeeper::new(container.clone())?;
        self.container.Children()?.InsertAtTop(container.clone())?;
        let slot = slot_keeper.tag();
        self.cells.push(Cell {
            slot_keeper,
            container,
            limit,
        });
        self.resize_cells(self.container.Size()?)?;
        Ok(slot)
    }

    fn resize_cells(&mut self, size: Vector2) -> crate::Result<()> {
        if self.orientation == RibbonOrientation::Stack {
            for cell in &self.cells {
                let content_size = size.clone() * cell.limit.content_ratio.clone();
                let content_offset = Vector3 {
                    X: (size.X - content_size.X) / 2.,
                    Y: (size.Y - content_size.Y) / 2.,
                    Z: 0.,
                };
                cell.container.SetSize(&content_size)?;
                cell.container.SetOffset(&content_offset)?;
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
                cell.container.SetSize(&size)?;
                cell.container.SetOffset(if hor {
                    Vector3 {
                        X: pos,
                        Y: 0.,
                        Z: 0.,
                    }
                } else {
                    Vector3 {
                        X: 0.,
                        Y: pos,
                        Z: 0.,
                    }
                })?;
                pos += sizes[i];
            }
        }
        Ok(())
    }
}

impl SendSlotEvent for Ribbon {
    fn send_size(&mut self, event: SlotSize) -> crate::Result<()> {
        self.resize_cells(event.0)?;
        for cell in &mut self.cells {
            cell.slot_keeper
                .send_size(SlotSize(cell.container.Size()?))?
        }
        Ok(())
    }

    fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
        for cell in &mut self.cells {
            let point = cell.translate_point(event.0)?;
            cell.slot_keeper
                .send_mouse_left_pressed(MouseLeftPressed(point))?
        }
        Ok(())
    }

    fn send_mouse_left_pressed_focused(
        &mut self,
        event: MouseLeftPressedFocused,
    ) -> crate::Result<()> {
        for cell in &mut self.cells {
            let point = cell.translate_point(event.0)?;
            if cell.is_translated_point_in_cell(point)? {
                return cell
                    .slot_keeper
                    .send_mouse_left_pressed_focused(MouseLeftPressedFocused(point));
            }
        }
        Ok(())
    }
}

pub struct RibbonKeeper(Keeper<Ribbon>);

impl RibbonKeeper {
    pub fn new(
        spawner: impl Spawn,
        compositor: Compositor,
        slot: SlotTag,
        orientation: RibbonOrientation,
    ) -> crate::Result<Self> {
        let keeper = Self(Keeper::new(Ribbon::new(compositor, slot, orientation)?));
        keeper.spawn_event_handlers(spawner)?;
        Ok(keeper)
    }
    pub fn tag(&self) -> RibbonTag {
        RibbonTag(self.0.tag())
    }
    pub fn get(&self) -> RwLockReadGuard<'_, Ribbon> {
        self.0.get()
    }
    pub fn get_mut(&self) -> RwLockWriteGuard<'_, Ribbon> {
        self.0.get_mut()
    }
    fn spawn_event_handlers(&self, spawner: impl Spawn) -> crate::Result<()> {
        let slot = self.0.get().slot.tag();
        let ribbon = self.tag();
        spawner.spawn({
            let slot = slot.clone();
            let mut ribbon = ribbon.clone();
            unwrap_err(async move {
                while let Some(size) = slot.on_size().next().await {
                    ribbon.send_size(size)?
                }
                Ok(())
            })
        })?;

        spawner.spawn({
            let slot = slot.clone();
            let mut ribbon = ribbon.clone();
            unwrap_err({
                async move {
                    while let Some(event) = slot.on_mouse_left_pressed().next().await {
                        ribbon.send_mouse_left_pressed(event)?
                    }
                    Ok(())
                }
            })
        })?;

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

#[derive(Clone, PartialEq)]
pub struct RibbonTag(Tag<Ribbon>);

impl RibbonTag {
    // pub fn send_event<T: Clone + Send + Sync + 'static>(&self, event: T) -> crate::Result<()> {
    //     self.0.call(|v| v.send_event(event))?
    // }
    pub fn add_cell(&self, limit: CellLimit) -> crate::Result<SlotTag> {
        self.0.call_mut(|v| v.add_cell(limit))?
    }
}

impl SendSlotEvent for RibbonTag {
    fn send_size(&mut self, event: SlotSize) -> crate::Result<()> {
        self.0.call_mut(|v| v.send_size(event))?
    }

    fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
        self.0.call_mut(|v| v.send_mouse_left_pressed(event))?
    }

    fn send_mouse_left_pressed_focused(
        &mut self,
        event: MouseLeftPressedFocused,
    ) -> crate::Result<()> {
        self.0
            .call_mut(|v| v.send_mouse_left_pressed_focused(event))?
    }
}
