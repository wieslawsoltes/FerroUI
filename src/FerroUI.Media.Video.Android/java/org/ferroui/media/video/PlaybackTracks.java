package org.ferroui.media.video;

import androidx.media3.common.C;
import androidx.media3.common.Format;
import androidx.media3.common.TrackGroup;
import androidx.media3.common.TrackSelectionOverride;
import androidx.media3.common.Tracks;
import androidx.media3.common.text.Cue;
import androidx.media3.common.text.CueGroup;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** The player looper owns track updates. The UI reads immutable snapshots. */
final class PlaybackTracks {
    private volatile List<Choice> choices = Collections.emptyList();
    private volatile String captions = "";
    private int nextId;

    static int type(int kind) { return kind == 0 ? C.TRACK_TYPE_AUDIO : C.TRACK_TYPE_TEXT; }

    void update(Tracks tracks) {
        List<Choice> previous = choices;
        List<Choice> current = new ArrayList<>();
        for (Tracks.Group group : tracks.getGroups()) {
            if (group.getType() != C.TRACK_TYPE_AUDIO && group.getType() != C.TRACK_TYPE_TEXT) continue;
            for (int i = 0; i < group.length && current.size() < 128; i++) {
                if (!group.isTrackSupported(i)) continue;
                TrackGroup nativeGroup = group.getMediaTrackGroup();
                int id = -1;
                for (Choice old : previous) {
                    if (old.index == i && old.group.equals(nativeGroup)) { id = old.id; break; }
                }
                if (id < 0) id = nextId++;
                Format format = group.getTrackFormat(i);
                String label = format.label;
                if (label == null || label.isEmpty()) label = format.language;
                if (label == null || label.isEmpty() || label.equals("und")) {
                    label = (group.getType() == C.TRACK_TYPE_AUDIO ? "Audio track " : "Caption track ") + (current.size() + 1);
                }
                current.add(new Choice(nativeGroup, i, id, label, group.isTrackSelected(i)));
            }
        }
        choices = current;
    }

    String list(int kind) {
        JSONArray result = new JSONArray();
        try {
            for (Choice choice : choices) if (choice.group.type == type(kind)) {
                result.put(new JSONObject().put("id", choice.id).put("label", choice.label)
                    .put("selected", choice.selected));
            }
        } catch (JSONException invalid) {
            return "[]";
        }
        return result.toString();
    }

    TrackSelectionOverride override(int kind, int id) {
        for (Choice choice : choices) if (choice.group.type == type(kind) && choice.id == id) {
            return new TrackSelectionOverride(choice.group, choice.index);
        }
        return null;
    }

    void updateCues(CueGroup group) {
        StringBuilder text = new StringBuilder();
        for (Cue cue : group.cues) {
            if (cue.text == null || cue.text.length() == 0) continue;
            if (text.length() > 0) text.append('\n');
            text.append(cue.text, 0, Math.min(cue.text.length(), 4096 - text.length()));
            if (text.length() >= 4096) break;
        }
        captions = text.toString();
    }

    String captions() { return captions; }
    void clearCues() { captions = ""; }

    private static final class Choice {
        final TrackGroup group;
        final int index, id;
        final String label;
        final boolean selected;

        Choice(TrackGroup group, int index, int id, String label, boolean selected) {
            this.group = group;
            this.index = index;
            this.id = id;
            this.label = label;
            this.selected = selected;
        }
    }
}
