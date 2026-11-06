# Gestures & glide typing

All touch handling lives in `ScreenKeyboard.Input.KeyboardTouchProcessor` (platform independent, unit tested). The
Uno control forwards pointer events to it; mouse, pen and touch behave the same.

## Taps and presses

| Gesture | Result |
|---|---|
| Tap | Types the key (on release). A preview bubble shows the character (`ShowKeyPreview`). |
| Fast typing with several fingers | *Rollover*: pressing a new key commits keys still held by other fingers. |
| Long press on a character | Opens the popup (accents, symbol and number hints). Slide to choose, release to type, slide far below to cancel. |
| Long press on shift | Caps lock. |
| Double tap on shift | Caps lock (`DoubleTapShiftForCapsLock`, `DoubleTapDelay`). |
| Hold shift while typing | Shift chording: upper case while held, back to lower case on release. |
| Hold delete / arrows | Key repeat (`KeyRepeatDelay`, `KeyRepeatInterval`). |
| Long press on space | `SpaceBarLongPress` (default: language picker). |
| Long press on enter / period | FlorisBoard's `~enter` / `~right` popups (emoji, clipboard, one-handed; punctuation). |

## Swipes

| Gesture | Setting | Default |
|---|---|---|
| Drag horizontally on the space bar | `SpaceBarSwipeLeft` / `SpaceBarSwipeRight` | Move the cursor (continuous, one character per `CursorStepDistance`) |
| Swipe up on the space bar | `SpaceBarSwipeUp` | Clipboard panel |
| Swipe left from delete | `DeleteKeySwipeLeft` | Select words while dragging, delete on release (`DeleteWordsPrecisely`) |
| Swipe up / down on the keyboard | `SwipeUp` / `SwipeDown` | Shift / hide keyboard |
| Swipe left / right on the keyboard | `SwipeLeft` / `SwipeRight` | Next / previous language (only when glide typing is off — with glide typing, horizontal movement starting on a letter types a word) |

Every setting accepts any `SwipeAction` (see [Configuration](configuration.md)), for example:

```csharp
settings.SwipeUp = SwipeAction.SwitchToEditingContext;
settings.SwipeDown = SwipeAction.HideKeyboard;
settings.SpaceBarSwipeUp = SwipeAction.SwitchToMediaContext;
settings.DeleteKeySwipeLeft = SwipeAction.DeleteCharactersPrecisely;
settings.SwipeDistanceThreshold = 40; // DIPs
```

## Glide (swipe) typing

Slide your finger across the letters of a word without lifting it. The trail is drawn in the theme's
`GlideTrailColor` and fades out after `GlideTrailDuration` ms; live candidates are shown in the smartbar while
gliding (`GlidePreview`). On release the best word is typed (a space is inserted automatically between consecutive
glided words) and the alternatives remain in the smartbar — tap one to replace the word.

The recognizer is a C# port of FlorisBoard's statistical classifier (SHARK²-style):

1. candidates are pruned by the keys closest to the start and end of the gesture and by gesture length;
2. the gesture and each candidate's ideal path (key centers, with loops for double letters) are resampled to 200 points;
3. the shape distance (normalized) and location distance are converted to Gaussian probabilities and combined with
   the word frequency.

Glide typing uses the dictionary of the active language plus learned words. It is available when a dictionary exists
for the language, the characters layout is shown and the field allows suggestions.

```csharp
settings.GlideTyping = true;
settings.GlideShowTrail = true;
settings.GlideTrailDuration = 250;
settings.GlidePreview = true;
```

The classifier can be used standalone:

```csharp
var classifier = new GlideTypingClassifier();
classifier.SetLayout(engine.Keyboard.CharacterKeys);  // or a Dictionary<char, KeyRect>
classifier.SetWords(WordDictionary.LoadBundledEnglish());
IReadOnlyList<string> words = classifier.Classify(points, maxSuggestionCount: 5);
```

## Feedback

`KeyboardEngine.FeedbackRequested` / `OnScreenKeyboard.FeedbackRequested` report `KeyPress`, `KeyPressDelete`,
`KeyPressSpace`, `KeyPressEnter`, `KeyPressFunction`, `LongPress`, `KeyRepeat` and `GestureStep`. Haptics are
performed automatically on Android and iOS (`HapticFeedback`, `HapticDuration`); play sounds yourself when
`SoundFeedback` is enabled.

## Custom pointer sources

Because gestures are processed in `KeyboardTouchProcessor`, you can feed it from any input source (e.g. a remote
touch panel or automated UI tests):

```csharp
var touch = keyboard.TouchProcessor;
touch.PointerDown(id, x, y);
touch.PointerMove(id, x2, y2);
touch.PointerUp(id, x2, y2);
```
