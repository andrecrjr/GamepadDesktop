use std::os::fd::AsFd;

use egui_overlay as egui;
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, EventQueue, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_shm, wl_surface},
};

use crate::keyboard::{self, KeyboardPosition, KeyboardSession, KeyboardSettings};

#[path = "keyboard_view.rs"]
mod keyboard_view;

pub struct WaylandKeyboardOverlay {
    connection: Connection,
    queue: EventQueue<OverlayState>,
    state: OverlayState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    output: Option<wl_output::WlOutput>,
    placement: Option<crate::placement::Rect>,
    #[cfg(test)]
    position_updates: usize,
}

impl WaylandKeyboardOverlay {
    pub fn new() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("Wayland indisponível: {error}"))?;
        let (globals, queue) = registry_queue_init(&connection)
            .map_err(|error| format!("Não foi possível consultar o compositor Wayland: {error}"))?;
        let handle = queue.handle();
        let compositor = CompositorState::bind(&globals, &handle)
            .map_err(|error| format!("wl_compositor indisponível: {error}"))?;
        let layer_shell = LayerShell::bind(&globals, &handle)
            .map_err(|_| "Este compositor Wayland não oferece wlr-layer-shell; teclado próprio sem foco indisponível.".to_owned())?;
        let shm = Shm::bind(&globals, &handle)
            .map_err(|error| format!("wl_shm indisponível: {error}"))?;
        let surface = compositor.create_surface(&handle);
        let layer = layer_shell.create_layer_surface(
            &handle,
            surface,
            Layer::Overlay,
            Some("gamepad-desktop-keyboard"),
            None,
        );
        let (surface_width, surface_height) = KeyboardSettings::default().surface_size();
        layer.set_anchor(Anchor::BOTTOM);
        layer.set_margin(0, 0, 20, 0);
        layer.set_size(surface_width, surface_height);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        let input_region = Region::new(&compositor)
            .map_err(|error| format!("Não foi possível criar a região sem mouse: {error}"))?;
        layer
            .wl_surface()
            .set_input_region(Some(input_region.wl_region()));
        layer.commit();

        let mut state = OverlayState {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &handle),
            output_state: OutputState::new(&globals, &handle),
            shm,
            layer,
            pool: None,
            context: egui::Context::default(),
            renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra).with_caching(true),
            session: KeyboardSession::default(),
            width: surface_width,
            height: surface_height,
            configured: false,
            dirty: true,
            position_dirty: true,
        };
        state.context.set_visuals(egui::Visuals::dark());
        state.pool = Some(
            SlotPool::new((surface_width * surface_height * 4) as usize, &state.shm)
                .map_err(|error| format!("Não foi possível alocar o teclado Wayland: {error}"))?,
        );

        let mut overlay = Self {
            connection,
            queue,
            state,
            compositor,
            layer_shell,
            output: None,
            placement: None,
            #[cfg(test)]
            position_updates: 0,
        };
        overlay
            .queue
            .roundtrip(&mut overlay.state)
            .map_err(|error| format!("O compositor não configurou a janela do teclado: {error}"))?;
        overlay.state.configured = true;
        overlay.state.draw()?;
        Ok(overlay)
    }

    pub fn update(&mut self, session: &KeyboardSession) -> Result<(), String> {
        if self.state.position_dirty
            || self.state.session.focused_input.map(|input| input.bounds)
                != session.focused_input.map(|input| input.bounds)
            || self.state.session.settings.surface_size() != session.settings.surface_size()
            || self.state.session.settings.position != session.settings.position
        {
            self.position(session)?;
            self.state.position_dirty = false;
        }
        if self.state.session != *session {
            self.state.session = session.clone();
            self.state.dirty = true;
        }
        self.connection
            .flush()
            .map_err(|error| format!("Falha ao atualizar a camada Wayland: {error}"))?;
        while self
            .queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| format!("Falha ao processar eventos Wayland: {error}"))?
            > 0
        {}

        if let Some(read) = self.queue.prepare_read() {
            let fd = self.connection.as_fd();
            let mut poll_fd = [PollFd::new(&fd, PollFlags::IN)];
            let timeout = Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if poll(&mut poll_fd, Some(&timeout))
                .map_err(|error| format!("Falha ao aguardar eventos Wayland: {error}"))?
                > 0
            {
                read.read()
                    .map_err(|error| format!("Falha ao ler eventos Wayland: {error}"))?;
            }
        }

        while self
            .queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| format!("Falha ao processar eventos Wayland: {error}"))?
            > 0
        {}
        if self.state.dirty && self.state.configured {
            self.state.draw()?;
        }
        Ok(())
    }

    fn position(&mut self, session: &KeyboardSession) -> Result<(), String> {
        #[cfg(test)]
        {
            self.position_updates += 1;
        }
        let outputs: Vec<_> = self
            .state
            .output_state
            .outputs()
            .filter_map(|output| {
                let info = self.state.output_state.info(&output)?;
                let (x, y) = info.logical_position?;
                let (width, height) = info.logical_size?;
                let rect = crate::placement::Rect {
                    x,
                    y,
                    width,
                    height,
                };
                rect.valid().then_some((output, rect))
            })
            .collect();
        let input = session.focused_input.map(|f| f.bounds);
        let selected = input
            .and_then(|input| {
                outputs
                    .iter()
                    .find(|(_, screen)| screen.contains_center(input))
                    .or_else(|| {
                        outputs
                            .iter()
                            .filter_map(|entry| {
                                entry.1.intersection(input).map(|visible| {
                                    (entry, i64::from(visible.width) * i64::from(visible.height))
                                })
                            })
                            .max_by_key(|(_, area)| *area)
                            .map(|(entry, _)| entry)
                    })
            })
            .or_else(|| {
                outputs
                    .iter()
                    .find(|(output, _)| Some(output) == self.output.as_ref())
            })
            .or_else(|| outputs.first());
        let Some((output, screen)) = selected else {
            // xdg-output geometry is optional. Keep configurable docking when
            // a compositor does not publish logical monitor coordinates.
            if self.state.session.settings != session.settings {
                let (width, height) = session.settings.surface_size();
                self.state.layer.set_anchor(
                    if session.settings.position == KeyboardPosition::Top {
                        Anchor::TOP
                    } else {
                        Anchor::BOTTOM
                    },
                );
                self.state.layer.set_margin(20, 0, 20, 0);
                self.state.layer.set_size(width, height);
                self.state.width = width;
                self.state.height = height;
                self.state.layer.commit();
            }
            return Ok(());
        };
        let rect = crate::placement::keyboard_rect(
            *screen,
            input,
            session.settings.surface_size(),
            session.settings.position == KeyboardPosition::Top,
        );
        if self.output.as_ref() != Some(output) {
            let layer = self.layer_shell.create_layer_surface(
                &self.queue.handle(),
                self.compositor.create_surface(&self.queue.handle()),
                Layer::Overlay,
                Some("gamepad-desktop-keyboard"),
                Some(output),
            );
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            let region = Region::new(&self.compositor).map_err(|e| e.to_string())?;
            layer
                .wl_surface()
                .set_input_region(Some(region.wl_region()));
            self.state.layer = layer;
            self.output = Some(output.clone());
            self.state.configured = false;
            self.placement = None;
        }
        if self.placement != Some(rect) {
            self.state.layer.set_anchor(Anchor::TOP | Anchor::LEFT);
            self.state.layer.set_exclusive_zone(-1);
            self.state
                .layer
                .set_margin(rect.y - screen.y, 0, 0, rect.x - screen.x);
            self.state
                .layer
                .set_size(rect.width as u32, rect.height as u32);
            self.state.width = rect.width as u32;
            self.state.height = rect.height as u32;
            self.state.dirty = true;
            self.state.layer.commit();
            self.placement = Some(rect);
        }
        Ok(())
    }
}

struct OverlayState {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    layer: LayerSurface,
    pool: Option<SlotPool>,
    context: egui::Context,
    renderer: EguiSoftwareRender,
    session: KeyboardSession,
    width: u32,
    height: u32,
    configured: bool,
    dirty: bool,
    position_dirty: bool,
}

impl OverlayState {
    fn draw(&mut self) -> Result<(), String> {
        let width = self.width.max(1);
        let height = self.height.max(1);
        let pool = self.pool.as_mut().expect("Wayland buffer pool initialized");
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .map_err(|error| format!("Falha ao criar o buffer do teclado: {error}"))?;

        if self.session.open {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                )),
                ..Default::default()
            };
            let output = self.context.run_ui(input, |ui| {
                keyboard_ui(ui, &self.session);
            });
            let paint_jobs = self
                .context
                .tessellate(output.shapes, output.pixels_per_point);
            let pixels = bytemuck::cast_slice_mut(canvas);
            let mut buffer_ref = BufferMutRef::new(pixels, width as usize, height as usize);
            self.renderer.render(
                &mut buffer_ref,
                &paint_jobs,
                &output.textures_delta,
                output.pixels_per_point,
            );
        } else {
            canvas.fill(0);
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.layer.wl_surface())
            .map_err(|error| format!("Falha ao anexar o teclado à camada: {error}"))?;
        self.layer.commit();
        self.dirty = false;
        Ok(())
    }
}

fn keyboard_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    keyboard_view::show(ui, session);
}

impl CompositorHandler for OverlayState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
        self.dirty = true;
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
        self.dirty = true;
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for OverlayState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.session.open = false;
        self.dirty = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if configure.new_size.0 > 0 {
            self.width = configure.new_size.0;
        }
        if configure.new_size.1 > 0 {
            self.height = configure.new_size.1;
        }
        self.configured = true;
        self.dirty = true;
    }
}

impl OutputHandler for OverlayState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.position_dirty = true;
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.position_dirty = true;
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.position_dirty = true;
    }
}

impl SeatHandler for OverlayState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ProvidesRegistryState for OverlayState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for OverlayState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(OverlayState);
delegate_layer!(OverlayState);
delegate_output!(OverlayState);
delegate_registry!(OverlayState);
delegate_seat!(OverlayState);
delegate_shm!(OverlayState);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{focused_input::FocusedInput, placement::Rect};

    #[test]
    #[ignore = "requires a Wayland compositor supporting wlr-layer-shell"]
    fn positioning_tracks_geometry_and_output_changes_without_repeating_idle_work() {
        let mut overlay = WaylandKeyboardOverlay::new().unwrap();
        let mut session = KeyboardSession::default();
        overlay.update(&session).unwrap();
        let mut expected = overlay.position_updates;
        overlay.update(&session).unwrap();
        assert_eq!(overlay.position_updates, expected, "idle update");

        session.open = true;
        session.move_selection(0, 1);
        session.settings.opacity = 200;
        overlay.update(&session).unwrap();
        assert_eq!(overlay.position_updates, expected, "visual changes only");

        session.focused_input = Some(FocusedInput {
            bounds: Rect {
                x: 100,
                y: 100,
                width: 240,
                height: 40,
            },
            screen: None,
        });
        overlay.update(&session).unwrap();
        expected += 1;
        assert_eq!(overlay.position_updates, expected, "focused input appeared");
        session.focused_input.as_mut().unwrap().bounds.y = 500;
        overlay.update(&session).unwrap();
        expected += 1;
        assert_eq!(overlay.position_updates, expected, "focused input moved");

        session.settings.width = 800;
        overlay.update(&session).unwrap();
        expected += 1;
        assert_eq!(overlay.position_updates, expected, "keyboard resized");
        session.settings.position = KeyboardPosition::Top;
        overlay.update(&session).unwrap();
        expected += 1;
        assert_eq!(overlay.position_updates, expected, "docking changed");

        let output = overlay.state.output_state.outputs().next().unwrap();
        for callback in [
            <OverlayState as OutputHandler>::new_output,
            <OverlayState as OutputHandler>::update_output,
            <OverlayState as OutputHandler>::output_destroyed,
        ] {
            callback(
                &mut overlay.state,
                &overlay.connection,
                &overlay.queue.handle(),
                output.clone(),
            );
            overlay.update(&session).unwrap();
            expected += 1;
            assert_eq!(overlay.position_updates, expected, "monitor event");
        }
        session.focused_input = None;
        overlay.update(&session).unwrap();
        expected += 1;
        assert_eq!(
            overlay.position_updates, expected,
            "focused input disappeared"
        );
        overlay.update(&session).unwrap();
        assert_eq!(
            overlay.position_updates, expected,
            "idle after geometry change"
        );
    }
}
