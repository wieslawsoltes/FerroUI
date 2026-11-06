using ScreenKeyboard.Engine;
using ScreenKeyboard.Glide;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Input;

/// <summary>Schedules delayed callbacks on the UI thread (abstracted for testability).</summary>
public interface IKeyboardScheduler
{
    /// <summary>Invokes <paramref name="action"/> after <paramref name="delay"/>; dispose the result to cancel.</summary>
    IDisposable Schedule(TimeSpan delay, Action action);
}

/// <summary>The long-press popup currently shown.</summary>
public sealed class KeyPopupState
{
    internal KeyPopupState(ComputedKey key, IReadOnlyList<TextKeyData> keys, IReadOnlyList<KeyRect> bounds, int selectedIndex)
    {
        Key = key;
        Keys = keys;
        Bounds = bounds;
        SelectedIndex = selectedIndex;
    }

    /// <summary>The key that opened the popup.</summary>
    public ComputedKey Key { get; }

    /// <summary>The popup keys.</summary>
    public IReadOnlyList<TextKeyData> Keys { get; }

    /// <summary>Bounds of each popup key in keyboard coordinates.</summary>
    public IReadOnlyList<KeyRect> Bounds { get; }

    /// <summary>Selected index or -1.</summary>
    public int SelectedIndex { get; internal set; }

    /// <summary>The bounds enclosing all popup keys.</summary>
    public KeyRect Frame
    {
        get
        {
            if (Bounds.Count == 0)
            {
                return default;
            }
            var left = Bounds.Min(b => b.Left);
            var top = Bounds.Min(b => b.Top);
            var right = Bounds.Max(b => b.Right);
            var bottom = Bounds.Max(b => b.Bottom);
            return new KeyRect(left, top, right - left, bottom - top);
        }
    }
}

/// <summary>
/// Converts raw pointer input on the keys area into keyboard actions: taps, multi-touch rollover,
/// long-press popups, key repeat, shift chording, glide typing, space bar cursor control, precise delete
/// selection and configurable swipe gestures. Platform independent: the UI forwards pointer events and
/// renders the visual state exposed through the events.
/// </summary>
public sealed class KeyboardTouchProcessor
{
    private readonly KeyboardEngine _engine;
    private readonly IKeyboardScheduler _scheduler;
    private readonly Dictionary<long, PointerState> _pointers = new();
    private readonly List<GesturePoint> _glideTrail = new();

    /// <summary>Creates a processor.</summary>
    public KeyboardTouchProcessor(KeyboardEngine engine, IKeyboardScheduler scheduler)
    {
        _engine = engine;
        _scheduler = scheduler;
    }

    /// <summary>Maximum number of popup keys per row.</summary>
    public int PopupMaxColumns { get; set; } = 6;

    /// <summary>Minimum Y coordinate popups may use (negative values allow overlapping the smartbar).</summary>
    public double PopupMinY { get; set; } = -48;

    /// <summary>Distance (DIPs) per cursor step when dragging on the space bar.</summary>
    public double CursorStepDistance { get; set; } = 14;

    /// <summary>Distance (DIPs) per step for precise deletion.</summary>
    public double DeleteStepDistance { get; set; } = 16;

    /// <summary>The key popup, if open.</summary>
    public KeyPopupState? Popup { get; private set; }

    /// <summary>Points of the current glide trail.</summary>
    public IReadOnlyList<GesturePoint> GlideTrail => _glideTrail;

    /// <summary>Keys currently pressed.</summary>
    public IEnumerable<ComputedKey> PressedKeys => _pointers.Values.Where(p => p.Key is not null && p.Phase == Phase.Pressed).Select(p => p.Key!);

    /// <summary>Raised when a key becomes pressed (visual state + preview).</summary>
    public event EventHandler<ComputedKey>? KeyPressed;

    /// <summary>Raised when a key is no longer pressed.</summary>
    public event EventHandler<ComputedKey>? KeyReleased;

    /// <summary>Raised when the popup opened, changed selection or closed (<see cref="Popup"/> is <c>null</c>).</summary>
    public event EventHandler? PopupChanged;

    /// <summary>Raised when the glide trail changed.</summary>
    public event EventHandler? GlideTrailChanged;

    /// <summary>Raised when a glide gesture ended.</summary>
    public event EventHandler? GlideEnded;

    private KeyboardSettings Settings => _engine.Settings;

    // ================================================================== pointer input

    /// <summary>Handles a pointer press in keys-area coordinates.</summary>
    public void PointerDown(long pointerId, double x, double y)
    {
        if (_pointers.ContainsKey(pointerId))
        {
            PointerCancel(pointerId);
        }

        // Rollover: a new finger commits keys still held by other fingers.
        foreach (var other in _pointers.Values.ToList())
        {
            if (other.Phase == Phase.Pressed && other.Key is { } k && k.Code != KeyCode.Shift && !other.Repeated && other.Key.Data.ProducesText)
            {
                other.CancelTimers();
                Release(other.Key);
                _engine.OnKeyUp(k.Data);
                other.Phase = Phase.Consumed;
            }
        }

        var key = _engine.Keyboard.HitTest(x, y);
        var state = new PointerState(pointerId, x, y, key);
        _pointers[pointerId] = state;
        if (key is null)
        {
            state.Phase = Phase.NoKey;
            return;
        }

        state.Phase = Phase.Pressed;
        Press(key);
        _engine.OnKeyDown(key.Data);

        if (key.Code == KeyCode.Delete && Settings.DeleteKeyLongPress != SwipeAction.DeleteCharacter)
        {
            state.LongPressTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Settings.LongPressDelay), () => OnLongPress(state));
        }
        else if (KeyboardEngine.IsRepeatable(key.Data))
        {
            state.RepeatTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Settings.KeyRepeatDelay), () => OnRepeat(state));
        }
        else
        {
            state.LongPressTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Settings.LongPressDelay), () => OnLongPress(state));
        }
    }

    /// <summary>Handles pointer movement.</summary>
    public void PointerMove(long pointerId, double x, double y)
    {
        if (!_pointers.TryGetValue(pointerId, out var state))
        {
            return;
        }
        state.LastX = x;
        state.LastY = y;
        var dx = x - state.StartX;
        var dy = y - state.StartY;
        var distance = Math.Sqrt(dx * dx + dy * dy);
        var threshold = Settings.SwipeDistanceThreshold;

        switch (state.Phase)
        {
            case Phase.Popup:
                UpdatePopupSelection(x, y);
                return;
            case Phase.Glide:
                _engine.AddGlidePoint(x, y);
                AddTrailPoint(x, y);
                return;
            case Phase.SpaceCursor:
            {
                var steps = (int)((x - state.AnchorX) / CursorStepDistance);
                if (steps != 0)
                {
                    state.AnchorX += steps * CursorStepDistance;
                    var action = steps < 0 ? Settings.SpaceBarSwipeLeft : Settings.SpaceBarSwipeRight;
                    _engine.ExecuteSwipeAction(action, Math.Abs(steps));
                }
                return;
            }
            case Phase.PreciseDelete:
            {
                var steps = (int)Math.Max(0, (state.StartX - x) / DeleteStepDistance);
                if (steps != state.Steps)
                {
                    state.Steps = steps;
                    var byWords = Settings.DeleteKeySwipeLeft is SwipeAction.DeleteWordsPrecisely or SwipeAction.SelectWordsPrecisely;
                    _engine.UpdatePreciseSelection(steps, byWords);
                }
                return;
            }
            case Phase.NoKey:
                if (distance > threshold)
                {
                    state.Phase = Phase.Swipe;
                }
                return;
            case Phase.Pressed when state.Key is { } key:
                if (distance <= threshold * 0.5 && !(key.IsCharacterKey && distance > key.TouchBounds.Width * 0.5))
                {
                    return;
                }
                BeginGesture(state, key, dx, dy, distance, threshold);
                return;
        }
    }

    private void BeginGesture(PointerState state, ComputedKey key, double dx, double dy, double distance, double threshold)
    {
        var horizontal = Math.Abs(dx) >= Math.Abs(dy);

        // Space bar: cursor control or swipe up.
        if (key.Data.IsSpace && distance > threshold * 0.5)
        {
            if (horizontal && (Settings.SpaceBarSwipeLeft != SwipeAction.NoAction || Settings.SpaceBarSwipeRight != SwipeAction.NoAction))
            {
                CancelKey(state);
                state.Phase = Phase.SpaceCursor;
                state.AnchorX = state.StartX;
                PointerMove(state.Id, state.LastX, state.LastY);
                return;
            }
            if (!horizontal && dy < 0 && distance > threshold && Settings.SpaceBarSwipeUp != SwipeAction.NoAction)
            {
                CancelKey(state);
                state.Phase = Phase.Consumed;
                _engine.ExecuteSwipeAction(Settings.SpaceBarSwipeUp);
                return;
            }
            return;
        }

        // Delete key: swipe left to select/delete precisely.
        if (key.Code == KeyCode.Delete && horizontal && dx < 0 && distance > threshold * 0.5)
        {
            switch (Settings.DeleteKeySwipeLeft)
            {
                case SwipeAction.DeleteCharactersPrecisely:
                case SwipeAction.DeleteWordsPrecisely:
                case SwipeAction.SelectCharactersPrecisely:
                case SwipeAction.SelectWordsPrecisely:
                    CancelKey(state);
                    state.Phase = Phase.PreciseDelete;
                    state.Steps = 0;
                    _engine.BeginPreciseSelection();
                    PointerMove(state.Id, state.LastX, state.LastY);
                    return;
                case SwipeAction.NoAction:
                    return;
                default:
                    CancelKey(state);
                    state.Phase = Phase.Consumed;
                    _engine.ExecuteSwipeAction(Settings.DeleteKeySwipeLeft);
                    return;
            }
        }

        // Character keys: glide typing.
        if (key.IsCharacterKey && _engine.CanGlide && _pointers.Count == 1 && distance > Math.Min(threshold, key.TouchBounds.Width * 0.6))
        {
            CancelKey(state);
            state.Phase = Phase.Glide;
            _glideTrail.Clear();
            _engine.BeginGlide(state.StartX, state.StartY);
            AddTrailPoint(state.StartX, state.StartY);
            _engine.AddGlidePoint(state.LastX, state.LastY);
            AddTrailPoint(state.LastX, state.LastY);
            return;
        }

        // Anything else: a swipe gesture once the threshold is exceeded.
        if (distance > threshold && key.Code != KeyCode.Shift)
        {
            CancelKey(state);
            state.Phase = Phase.Swipe;
        }
    }

    /// <summary>Handles a pointer release.</summary>
    public void PointerUp(long pointerId, double x, double y)
    {
        if (!_pointers.TryGetValue(pointerId, out var state))
        {
            return;
        }
        _pointers.Remove(pointerId);
        state.CancelTimers();
        state.LastX = x;
        state.LastY = y;

        switch (state.Phase)
        {
            case Phase.Pressed when state.Key is { } key:
                Release(key);
                if (!state.Repeated || key.Code == KeyCode.Shift)
                {
                    _engine.OnKeyUp(key.Data);
                }
                break;
            case Phase.Popup:
            {
                var popup = Popup;
                ClosePopup();
                if (state.Key is { } k)
                {
                    _engine.OnKeyCancel(k.Data);
                }
                if (popup is { SelectedIndex: >= 0 } && popup.SelectedIndex < popup.Keys.Count)
                {
                    var selected = popup.Keys[popup.SelectedIndex];
                    _engine.InputKey(selected);
                }
                break;
            }
            case Phase.Glide:
                _engine.FinishGlide(commit: true);
                GlideEnded?.Invoke(this, EventArgs.Empty);
                break;
            case Phase.PreciseDelete:
            {
                var delete = Settings.DeleteKeySwipeLeft is SwipeAction.DeleteCharactersPrecisely or SwipeAction.DeleteWordsPrecisely;
                _engine.EndPreciseSelection(delete);
                break;
            }
            case Phase.Swipe:
                ExecuteSwipe(state);
                break;
        }
    }

    /// <summary>Cancels a pointer (e.g. capture lost).</summary>
    public void PointerCancel(long pointerId)
    {
        if (!_pointers.TryGetValue(pointerId, out var state))
        {
            return;
        }
        _pointers.Remove(pointerId);
        state.CancelTimers();
        switch (state.Phase)
        {
            case Phase.Pressed when state.Key is { } key:
                Release(key);
                _engine.OnKeyCancel(key.Data);
                break;
            case Phase.Popup:
                ClosePopup();
                break;
            case Phase.Glide:
                _engine.FinishGlide(commit: false);
                GlideEnded?.Invoke(this, EventArgs.Empty);
                break;
            case Phase.PreciseDelete:
                _engine.CancelPreciseSelection();
                break;
        }
    }

    /// <summary>Cancels all active pointers.</summary>
    public void CancelAll()
    {
        foreach (var id in _pointers.Keys.ToList())
        {
            PointerCancel(id);
        }
    }

    /// <summary>Clears the glide trail (call after the fade-out animation).</summary>
    public void ClearGlideTrail()
    {
        _glideTrail.Clear();
        GlideTrailChanged?.Invoke(this, EventArgs.Empty);
    }

    // ================================================================== helpers

    private void ExecuteSwipe(PointerState state)
    {
        var dx = state.LastX - state.StartX;
        var dy = state.LastY - state.StartY;
        if (Math.Sqrt(dx * dx + dy * dy) < Settings.SwipeDistanceThreshold)
        {
            return;
        }
        var action = Math.Abs(dx) >= Math.Abs(dy)
            ? dx < 0 ? Settings.SwipeLeft : Settings.SwipeRight
            : dy < 0 ? Settings.SwipeUp : Settings.SwipeDown;
        _engine.ExecuteSwipeAction(action);
    }

    private void OnRepeat(PointerState state)
    {
        if (!_pointers.ContainsKey(state.Id) || state.Phase != Phase.Pressed || state.Key is null)
        {
            return;
        }
        state.Repeated = true;
        _engine.OnKeyRepeat(state.Key.Data);
        state.RepeatTimer = _scheduler.Schedule(TimeSpan.FromMilliseconds(Settings.KeyRepeatInterval), () => OnRepeat(state));
    }

    private void OnLongPress(PointerState state)
    {
        if (!_pointers.ContainsKey(state.Id) || state.Phase != Phase.Pressed || state.Key is not { } key)
        {
            return;
        }
        switch (key.Code)
        {
            case KeyCode.Shift:
                state.Repeated = true;
                _engine.FeedbackLongPress();
                _engine.SetShiftState(ShiftState.CapsLock);
                return;
            case KeyCode.Space:
                if (Settings.SpaceBarLongPress != SwipeAction.NoAction)
                {
                    CancelKey(state);
                    state.Phase = Phase.Consumed;
                    _engine.FeedbackLongPress();
                    _engine.ExecuteSwipeAction(Settings.SpaceBarLongPress);
                }
                return;
            case KeyCode.Delete:
                if (Settings.DeleteKeyLongPress == SwipeAction.DeleteCharacter)
                {
                    OnRepeat(state);
                }
                else
                {
                    CancelKey(state);
                    state.Phase = Phase.Consumed;
                    _engine.ExecuteSwipeAction(Settings.DeleteKeyLongPress);
                }
                return;
        }
        if (key.PopupKeys.Count == 0)
        {
            return;
        }
        state.CancelTimers();
        Release(key);
        state.Phase = Phase.Popup;
        _engine.FeedbackLongPress();
        OpenPopup(key);
    }

    private void OpenPopup(ComputedKey key)
    {
        var keyboard = _engine.Keyboard;
        var keys = key.PopupKeys;
        var (bounds, selected) = ArrangePopup(key, keys.Count, key.PopupDefaultIndex, keyboard.Width, PopupMaxColumns, PopupMinY);
        Popup = new KeyPopupState(key, keys, bounds, selected);
        PopupChanged?.Invoke(this, EventArgs.Empty);
    }

    private void ClosePopup()
    {
        if (Popup is null)
        {
            return;
        }
        Popup = null;
        PopupChanged?.Invoke(this, EventArgs.Empty);
    }

    private void UpdatePopupSelection(double x, double y)
    {
        if (Popup is not { } popup || popup.Bounds.Count == 0)
        {
            return;
        }
        var key = popup.Key.VisibleBounds;
        int selected;
        if (y > key.Bottom + key.Height)
        {
            // Dragged far below the key: no selection (release cancels).
            selected = -1;
        }
        else
        {
            // Pick the closest popup key; when below the popup use the bottom row.
            var bottomRowTop = popup.Bounds.Max(b => b.Top);
            var candidates = Enumerable.Range(0, popup.Bounds.Count).ToList();
            if (y >= popup.Frame.Bottom)
            {
                candidates = candidates.Where(i => Math.Abs(popup.Bounds[i].Top - bottomRowTop) < 0.5).ToList();
            }
            selected = candidates
                .OrderBy(i => popup.Bounds[i].CenterDistanceSquared(x, Math.Min(y, popup.Frame.Bottom - 1)))
                .First();
        }
        if (selected != popup.SelectedIndex)
        {
            popup.SelectedIndex = selected;
            _engine.FeedbackGestureStep();
            PopupChanged?.Invoke(this, EventArgs.Empty);
        }
    }

    /// <summary>
    /// Computes popup key bounds above <paramref name="key"/>. The default key is placed directly above the key,
    /// further keys alternate to the right and left (FlorisBoard style), overflowing into additional rows.
    /// </summary>
    public static (IReadOnlyList<KeyRect> Bounds, int SelectedIndex) ArrangePopup(ComputedKey key, int count, int defaultIndex, double keyboardWidth, int maxColumns = 6, double minY = double.MinValue)
    {
        var result = new KeyRect[count];
        if (count == 0)
        {
            return (result, -1);
        }
        var anchor = key.VisibleBounds;
        var cellW = Math.Max(anchor.Width, 28);
        var cellH = Math.Max(anchor.Height, 28);
        var columns = Math.Min(count, Math.Max(1, maxColumns));
        var rows = (int)Math.Ceiling(count / (double)columns);

        // Column of the anchor: keys at the edges open their popup towards the center.
        var anchorCol = keyboardWidth > 0 && anchor.CenterX < keyboardWidth / 3 ? 0
            : keyboardWidth > 0 && anchor.CenterX > keyboardWidth * 2 / 3 ? columns - 1
            : (columns - 1) / 2;
        var left = anchor.CenterX - cellW / 2 - anchorCol * cellW;
        if (keyboardWidth > 0)
        {
            left = Math.Max(0, Math.Min(left, keyboardWidth - columns * cellW));
        }
        var top = anchor.Top - rows * cellH;
        if (top < minY)
        {
            top = minY;
        }

        // Column visiting order: anchor, anchor+1, anchor-1, anchor+2, ...
        var order = new List<int> { anchorCol };
        for (var d = 1; order.Count < columns; d++)
        {
            if (anchorCol + d < columns) order.Add(anchorCol + d);
            if (anchorCol - d >= 0 && order.Count < columns) order.Add(anchorCol - d);
        }

        var items = Enumerable.Range(0, count).ToList();
        if (defaultIndex > 0 && defaultIndex < count)
        {
            items.Remove(defaultIndex);
            items.Insert(0, defaultIndex);
        }
        var index = 0;
        for (var r = 0; r < rows && index < count; r++)
        {
            var y = top + (rows - 1 - r) * cellH; // row 0 is the bottom row
            foreach (var col in order)
            {
                if (index >= count)
                {
                    break;
                }
                result[items[index]] = new KeyRect(left + col * cellW, y, cellW, cellH);
                index++;
            }
        }
        return (result, Math.Max(0, defaultIndex));
    }

    private void AddTrailPoint(double x, double y)
    {
        if (!Settings.GlideShowTrail)
        {
            return;
        }
        _glideTrail.Add(new GesturePoint(x, y));
        if (_glideTrail.Count > 400)
        {
            _glideTrail.RemoveAt(0);
        }
        GlideTrailChanged?.Invoke(this, EventArgs.Empty);
    }

    private void CancelKey(PointerState state)
    {
        state.CancelTimers();
        if (state.Key is { } key && state.Phase == Phase.Pressed)
        {
            Release(key);
            _engine.OnKeyCancel(key.Data);
        }
    }

    private void Press(ComputedKey key) => KeyPressed?.Invoke(this, key);

    private void Release(ComputedKey key) => KeyReleased?.Invoke(this, key);

    private enum Phase
    {
        NoKey,
        Pressed,
        Popup,
        Glide,
        SpaceCursor,
        PreciseDelete,
        Swipe,
        Consumed,
    }

    private sealed class PointerState(long id, double x, double y, ComputedKey? key)
    {
        public long Id { get; } = id;
        public double StartX { get; } = x;
        public double StartY { get; } = y;
        public double LastX { get; set; } = x;
        public double LastY { get; set; } = y;
        public double AnchorX { get; set; } = x;
        public ComputedKey? Key { get; } = key;
        public Phase Phase { get; set; }
        public bool Repeated { get; set; }
        public int Steps { get; set; }
        public IDisposable? LongPressTimer { get; set; }
        public IDisposable? RepeatTimer { get; set; }

        public void CancelTimers()
        {
            LongPressTimer?.Dispose();
            LongPressTimer = null;
            RepeatTimer?.Dispose();
            RepeatTimer = null;
        }
    }
}
