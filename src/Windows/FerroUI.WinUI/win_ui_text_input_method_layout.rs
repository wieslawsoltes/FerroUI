// Native layout requests retain one semantic owner and input session. Neither
// a client callback nor a COM property access may redirect the old request to
// whichever control happens to be focused when that callback returns.
struct LayoutRequestState {
    layout_generation: u64,
    client_generation: u64,
    composition_generation: u64,
    has_focus: bool,
    is_composing: bool,
    client: Rc<dyn TextInputMethodClient>,
    connection: Option<Rc<TextInputMethodConnection>>,
    composition: Option<TextInputMethodComposition>,
    client_revision: Option<u64>,
    cursor_rect: FerroRect,
}

struct LayoutOwner {
    input_root: Rc<dyn IInputRoot>,
    root: Ref<Visual>,
    owner: Ref<Visual>,
    owner_bounds: FerroRect,
    owner_to_root: Matrix,
    control_bounds: FerroRect,
}

impl WinUITextInputMethod {
    fn invalidate_layout_request(&self) {
        self.layout_generation
            .set(self.layout_generation.get().wrapping_add(1));
    }

    fn begin_layout_request(&self) -> Option<LayoutRequestState> {
        self.invalidate_layout_request();
        let client = self.client.borrow().clone()?;
        let mut state = LayoutRequestState {
            layout_generation: self.layout_generation.get(),
            client_generation: self.client_generation.get(),
            composition_generation: self.composition_generation.get(),
            has_focus: self.has_focus.get(),
            is_composing: self.is_composing.get(),
            client,
            connection: self.connection.borrow().clone(),
            composition: self.exact_composition.borrow().clone(),
            client_revision: None,
            cursor_rect: self.cursor_rect.get(),
        };
        state.client_revision = state.client.input_method_state_revision();
        self.layout_request_is_current(&state).then_some(state)
    }

    fn layout_request_epoch_is_current(&self, state: &LayoutRequestState) -> bool {
        self.layout_generation.get() == state.layout_generation
            && self.client_generation.get() == state.client_generation
            && self.composition_generation.get() == state.composition_generation
            && self.has_focus.get() == state.has_focus
            && self.is_composing.get() == state.is_composing
            && self.cursor_rect.get() == state.cursor_rect
            && self
                .client
                .borrow()
                .as_ref()
                .is_some_and(|client| Rc::ptr_eq(client, &state.client))
    }

    fn layout_request_is_current(&self, state: &LayoutRequestState) -> bool {
        if !self.layout_request_epoch_is_current(state)
            || state
                .connection
                .as_ref()
                .is_some_and(|connection| !connection.is_current())
            || state
                .composition
                .as_ref()
                .is_some_and(|composition| !composition.is_current())
        {
            return false;
        }
        // The intrinsic revision observes edits whose public notifications are
        // still deferred. Both it and composition validation can call clients;
        // recheck the native ownership epoch after those calls.
        let revision = state.client.input_method_state_revision();
        self.layout_request_epoch_is_current(state) && revision == state.client_revision
    }

    fn layout_owner_visual(&self, state: &LayoutRequestState) -> Option<Ref<Visual>> {
        if !self.layout_request_epoch_is_current(state) {
            return None;
        }
        if let Some(connection) = &state.connection {
            // The manager connection carries the original semantic control.
            // Its presenter may be only a small child inside that control.
            return connection.owner().map(|owner| owner.upcast());
        }
        let owner = state
            .client
            .input_method_owner()
            .map(|owner| owner.upcast());
        if !self.layout_request_epoch_is_current(state) {
            return None;
        }
        let owner = owner.or_else(|| state.client.text_view_visual());
        self.layout_request_epoch_is_current(state)
            .then_some(owner)
            .flatten()
    }

    fn capture_layout_owner(&self, state: &LayoutRequestState) -> Option<LayoutOwner> {
        let owner = self.layout_owner_visual(state)?;
        let input_root = (self.get_input_root)()?;
        if !self.layout_request_is_current(state) {
            return None;
        }
        let root: Ref<Visual> = input_root.try_root_element()?.upcast();
        if !owner.is_attached_to_visual_tree()
            || !root.is_attached_to_visual_tree()
            || owner.visual_root() != root.visual_root()
        {
            return None;
        }
        let owner_bounds = owner.bounds();
        let owner_to_root = owner.transform_to_visual(&root)?;
        let control_bounds =
            FerroRect::from_size(owner_bounds.size()).transform_to_aabb(owner_to_root);
        let layout = LayoutOwner {
            input_root,
            root,
            owner,
            owner_bounds,
            owner_to_root,
            control_bounds,
        };
        self.layout_owner_is_current(state, &layout)
            .then_some(layout)
    }

    fn layout_owner_is_current(&self, state: &LayoutRequestState, layout: &LayoutOwner) -> bool {
        if !self.layout_request_is_current(state)
            || self.layout_owner_visual(state).as_ref() != Some(&layout.owner)
        {
            return false;
        }
        let Some(input_root) = (self.get_input_root)() else {
            return false;
        };
        if !Rc::ptr_eq(&input_root, &layout.input_root) {
            return false;
        }
        let root = input_root
            .try_root_element()
            .map(|root| root.upcast::<Visual>());
        // The client's revision accessor can invoke application code. Finish
        // it before comparing the captured geometry, then close with only the
        // adapter's callback-free epoch check. A resize in that last accessor
        // must not validate the old control rectangle.
        self.layout_request_is_current(state)
            && root.as_ref() == Some(&layout.root)
            && layout.owner.is_attached_to_visual_tree()
            && layout.root.is_attached_to_visual_tree()
            && layout.owner.visual_root() == layout.root.visual_root()
            && layout.owner.bounds() == layout.owner_bounds
            && layout.owner.transform_to_visual(&layout.root) == Some(layout.owner_to_root)
            && self.layout_request_epoch_is_current(state)
    }

    fn layout_sender_is_current(&self, sender: &ICoreTextEditContext) -> bool {
        self.edit_context
            .borrow()
            .as_ref()
            .is_some_and(|context| std::ptr::eq(&**context, sender))
    }

    fn layout_text_geometry_is_current(
        &self,
        state: &LayoutRequestState,
        owner: &LayoutOwner,
        text: &CoreTextTextGeometry,
    ) -> bool {
        // Run ownership/revision accessors before the geometry guard. The rich
        // guard ends with cached identity checks; its Base wrapper also rejects
        // visual changes made while it is validating the captured layout.
        self.layout_owner_is_current(state, owner)
            && text.is_current()
            && self.layout_request_epoch_is_current(state)
    }

    fn query_layout_text_geometry(
        &self,
        state: &LayoutRequestState,
        owner: &LayoutOwner,
        read_range: impl FnOnce() -> Result<CoreTextRange, HResult>,
    ) -> Result<Option<CoreTextTextGeometry>, HResult> {
        if !self.layout_owner_is_current(state, owner) {
            return Ok(None);
        }
        let range = read_range()?;
        if !self.layout_owner_is_current(state, owner) {
            return Ok(None);
        }
        let range = TextSelection::new(range.start_caret_position, range.end_caret_position);
        let Ok(text) = CoreTextTextGeometry::query(&*state.client, range, state.cursor_rect) else {
            return Ok(None);
        };
        Ok(self
            .layout_text_geometry_is_current(state, owner, &text)
            .then_some(text))
    }

    fn on_layout_requested(
        &self,
        sender: &ICoreTextEditContext,
        args: &ICoreTextLayoutRequestedEventArgs,
    ) {
        if !self.layout_sender_is_current(sender) {
            return;
        }
        let Some(state) = self.begin_layout_request() else {
            return;
        };
        let result = (|| -> Result<(), HResult> {
            let request = args.request()?.ok_or(HResult::POINTER)?;
            if request.is_canceled()? != 0 || !self.layout_request_is_current(&state) {
                return Ok(());
            }
            let Some(owner) = self.capture_layout_owner(&state) else {
                return Ok(());
            };
            let Some(text) = self.query_layout_text_geometry(&state, &owner, || request.range())?
            else {
                return Ok(());
            };
            if request.is_canceled()? != 0
                || !self.layout_sender_is_current(sender)
                || !self.layout_text_geometry_is_current(&state, &owner, &text)
            {
                return Ok(());
            }
            let geometry = self.compute_screen_layout(&state, &owner, &text)?;
            let bounds = request.layout_bounds()?.ok_or(HResult::POINTER)?;
            // Requested visible-story text and the complete semantic control
            // remain separate. These are never candidate-window rectangles.
            geometry.publish(
                || {
                    request.is_canceled().is_ok_and(|canceled| canceled == 0)
                        && self.layout_sender_is_current(sender)
                        && self.layout_text_geometry_is_current(&state, &owner, &text)
                },
                |rect| bounds.set_text_bounds(rect),
                |rect| bounds.set_control_bounds(rect),
            )?;
            Ok(())
        })();
        if let Err(error) = result {
            if error != HResult::ABORT {
                if let Some(logger) =
                    Logger::try_get(LogEventLevel::Verbose, LogArea::WINUI_PLATFORM)
                {
                    logger.log(
                        Some(self),
                        &format!("LayoutRequested couldn't compute screen bounds: {error}"),
                    );
                }
            }
        }
    }

    fn compute_screen_layout(
        &self,
        state: &LayoutRequestState,
        owner: &LayoutOwner,
        text: &CoreTextTextGeometry,
    ) -> Result<CoreTextLayoutBounds, HResult> {
        let ensure_current = || {
            if self.layout_text_geometry_is_current(state, owner, text) {
                Ok(())
            } else {
                Err(HResult::ABORT)
            }
        };
        ensure_current()?;
        let panel = self.panel.upgrade().ok_or(HResult::POINTER)?;
        let element = panel.as_ui_element();
        let transform = element.transform_to_visual(None)?.ok_or(HResult::POINTER)?;
        ensure_current()?;
        let xaml_root = element.xaml_root()?.ok_or(HResult::POINTER)?;
        ensure_current()?;
        let scale = xaml_root.rasterization_scale()?;
        ensure_current()?;
        let xaml_root2 = xaml_root.cast::<IXamlRoot2>()?;
        ensure_current()?;
        let island = xaml_root2
            .content_island_environment()?
            .ok_or(HResult::POINTER)?;
        ensure_current()?;
        let window_id = island.app_window_id()?;
        ensure_current()?;
        let hwnd = get_window_from_window_id(window_id);
        if hwnd == 0 {
            return Err(HResult::HANDLE);
        }
        let origin = client_origin_on_screen(hwnd).ok_or(HResult::HANDLE)?;
        ensure_current()?;
        match text {
            CoreTextTextGeometry::Requested(geometry) => {
                CoreTextLayoutBounds::from_owner_fragments(
                    geometry.fragments(),
                    FerroRect::from_size(owner.owner_bounds.size()),
                    owner.owner_to_root,
                    scale,
                    origin,
                    |point| {
                        ensure_current()?;
                        let transformed = transform.transform_point(point)?;
                        ensure_current()?;
                        Ok(transformed)
                    },
                )
            }
            CoreTextTextGeometry::LegacyCaret(caret) => CoreTextLayoutBounds::from_root_bounds(
                *caret,
                owner.control_bounds,
                scale,
                origin,
                |rect| {
                    ensure_current()?;
                    let transformed = transform.transform_bounds(rect)?;
                    ensure_current()?;
                    Ok(transformed)
                },
            ),
        }
    }
}
