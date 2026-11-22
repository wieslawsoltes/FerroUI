#pragma once

// AppKit's input context calls the same client method for keyboard text,
// composition commits, and text supplied outside keyDown.
class FrnTextInputDispatch
{
    enum class Origin { TextSystem, Keyboard, InputMethod };
    Origin _origin = Origin::TextSystem;

public:
    class KeyEvent
    {
        FrnTextInputDispatch& _dispatch;
        Origin _previous;

    public:
        KeyEvent(FrnTextInputDispatch& dispatch, bool inputMethod)
            : _dispatch(dispatch), _previous(dispatch._origin)
        {
            dispatch._origin = inputMethod ? Origin::InputMethod : Origin::Keyboard;
        }

        KeyEvent(const KeyEvent&) = delete;
        KeyEvent& operator=(const KeyEvent&) = delete;

        ~KeyEvent() { _dispatch._origin = _previous; }
    };

    void MarkComposition()
    {
        if (_origin == Origin::Keyboard)
            _origin = Origin::InputMethod;
    }

    bool IsInputMethodCommit(bool hasMarkedText, bool hasReplacementRange) const
    {
        return _origin != Origin::Keyboard || hasMarkedText || hasReplacementRange;
    }
};
