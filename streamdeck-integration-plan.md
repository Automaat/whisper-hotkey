# Stream Deck Integration Plan

## Overview
Add Stream Deck button trigger support alongside existing hotkey. Two-component system: Rust daemon + Stream Deck plugin (JavaScript). Supports simultaneous hotkey + Stream Deck operation.

## Problem: HID Exclusive Access
**Direct HID approach BLOCKED:** USB HID devices allow only one app at a time. Elgato app running → direct `streamdeck` crate integration conflicts.

**Solution:** Official SDK plugin approach - works WITH Elgato app.

---

## Architecture

### Components
```
Stream Deck Hardware (USB HID)
    ↓ exclusive access
Elgato Stream Deck App (required)
    ↓ WebSocket protocol
whisper-hotkey Plugin (.sdPlugin)
    ↓ HTTP POST
whisper-hotkey Daemon (localhost:7777)
    ↓
Audio → Whisper → Text Insertion
```

### Event Flow
1. User presses button → `keyDown` event
2. Plugin: `POST http://localhost:7777/trigger/start`
3. Daemon starts recording
4. User releases → `keyUp` event
5. Plugin: `POST http://localhost:7777/trigger/stop`
6. Daemon transcribes + inserts text

---

## Technology Stack

### Daemon (Rust)
```toml
[dependencies]
axum = "0.7"              # HTTP server
tokio = { version = "1", features = ["rt-multi-thread"] }
serde_json = "1.0"
```

### Plugin (JavaScript)
```bash
npm install -g @elgato/cli@latest
streamdeck create
```

---

## HTTP API Design

### Endpoints

**POST /trigger/start**
- Starts recording (= hotkey press)
- Returns: `200 OK` or `409 Conflict` (already recording)

**POST /trigger/stop**
- Stops + transcribes (= hotkey release)
- Returns: `200 OK` or `400 Bad Request` (not recording)

**GET /health**
- Returns: `{"status": "ready", "model_loaded": true}`

**GET /status**
- Returns: `{"state": "idle|recording|transcribing", "duration_ms": 0}`

---

## Implementation Phases

### Phase 1: Daemon HTTP Server (2-3 hrs)

**File:** `src/http_server.rs`

```rust
use axum::{Router, routing::{post, get}, Json};

async fn trigger_start() -> Json<Response> {
    // Call existing hotkey press handler
}

async fn trigger_stop() -> Json<Response> {
    // Call existing hotkey release handler
}

pub async fn run_server(trigger_tx: Sender<TriggerEvent>) {
    let app = Router::new()
        .route("/trigger/start", post(trigger_start))
        .route("/trigger/stop", post(trigger_stop))
        .route("/health", get(health_check));

    axum::Server::bind(&"127.0.0.1:7777".parse()?)
        .serve(app.into_make_service())
        .await?;
}
```

**Main.rs:**
```rust
// Unified trigger handling
enum TriggerEvent {
    HotkeyPress,
    HotkeyRelease,
    HttpStart,
    HttpStop,
}

// Spawn HTTP server
tokio::spawn(http_server::run_server(trigger_tx.clone()));
```

**Config:**
```toml
[http]
enabled = true
port = 7777
bind = "127.0.0.1"  # localhost only
```

**Validation:**
```bash
cargo run &
curl -X POST http://localhost:7777/trigger/start
curl -X POST http://localhost:7777/trigger/stop
```

---

### Phase 2: Stream Deck Plugin (2-3 hrs)

**Structure:**
```
com.whisperhotkey.streamdeck.sdPlugin/
├── manifest.json
├── plugin.js
├── imgs/
│   ├── action.png
│   └── category.png
└── README.md
```

**manifest.json:**
```json
{
  "Name": "Whisper Hotkey",
  "Version": "1.0.0",
  "Author": "whisper-hotkey",
  "Actions": [
    {
      "Name": "Voice to Text",
      "UUID": "com.whisperhotkey.streamdeck.voicetotext",
      "Icon": "imgs/action",
      "States": [
        { "Image": "imgs/idle" },
        { "Image": "imgs/recording" }
      ]
    }
  ]
}
```

**plugin.js:**
```javascript
import streamDeck from '@elgato/streamdeck';

const DAEMON_URL = 'http://localhost:7777';

streamDeck.actions.registerAction(new class VoiceToText {
  async onKeyDown(ev) {
    try {
      await fetch(`${DAEMON_URL}/trigger/start`, { method: 'POST' });
      ev.action.setState(1); // Recording icon
    } catch (err) {
      ev.action.showAlert(); // Red X
    }
  }

  async onKeyUp(ev) {
    try {
      await fetch(`${DAEMON_URL}/trigger/stop`, { method: 'POST' });
      ev.action.setState(0); // Idle icon
    } catch (err) {
      ev.action.showAlert();
    }
  }

  async onWillAppear(ev) {
    // Health check on button load
    const health = await fetch(`${DAEMON_URL}/health`);
    if (!health.ok) ev.action.showAlert();
  }
});

streamDeck.connect();
```

**Build:**
```bash
cd streamdeck-plugin
npm install
streamdeck pack com.whisperhotkey.streamdeck.sdPlugin
```

**Install:** Double-click `.streamDeckPlugin` file

**Validation:**
- Press button → recording starts
- Release → transcription + insertion
- Daemon offline → red X

---

### Phase 3: Visual Feedback (1 hr)

**Icons:** `idle.png`, `recording.png` (128x128, 144x144, 288x288)

**Title display:**
```javascript
onKeyDown(ev) {
  ev.action.setTitle("🔴 Recording...");
}

onKeyUp(ev) {
  ev.action.setTitle("✍️ Transcribing...");
  setTimeout(() => ev.action.setTitle(""), 2000);
}
```

---

### Phase 4: Error Handling (1 hr)

**Daemon health check:**
```javascript
async onWillAppear(ev) {
  setInterval(async () => {
    try {
      await fetch(`${DAEMON_URL}/health`);
      ev.action.setTitle("✓ Ready");
    } catch {
      ev.action.setTitle("⚠️ Offline");
      ev.action.showAlert();
    }
  }, 5000);
}
```

**Timeout protection:**
```rust
// Daemon: auto-stop after 60s
const MAX_RECORDING_MS: u64 = 60_000;
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_millis(MAX_RECORDING_MS)).await;
    if state.is_recording() {
        warn!("Force-stop after timeout");
        stop_recording().await;
    }
});
```

---

## Distribution

### Package
```
whisper-hotkey-v1.0.0/
├── whisper-hotkey                   # Daemon binary
├── whisper-hotkey.streamDeckPlugin  # Plugin installer
├── install.sh
└── README.md
```

**install.sh:**
```bash
#!/bin/bash
sudo cp whisper-hotkey /usr/local/bin/
sudo chmod +x /usr/local/bin/whisper-hotkey
open whisper-hotkey.streamDeckPlugin
echo "✓ Installed. Configure ~/.whisper-hotkey.toml"
```

**GitHub Release:** Universal binary + plugin

---

## Configuration

**~/.whisper-hotkey.toml:**
```toml
[triggers]
hotkey_enabled = true
streamdeck_enabled = true

[hotkey]
modifiers = ["Command", "Shift"]
key = "V"

[http]
enabled = true      # Required for Stream Deck
port = 7777
bind = "127.0.0.1"  # Security: localhost only
```

---

## Performance Impact

**HTTP overhead:**
- Request: ~5-10ms (localhost)
- JSON parsing: <1ms
- **Total added latency: 10-20ms** (acceptable)

**Idle resources:**
- HTTP server: ~2MB RAM
- CPU: <0.1%

---

## Testing Checklist

### Daemon
- [ ] HTTP server starts on 7777
- [ ] `/health` returns 200
- [ ] `/trigger/start` starts recording
- [ ] `/trigger/stop` transcribes + inserts
- [ ] Hotkey works simultaneously
- [ ] 409 if start during recording
- [ ] 400 if stop when not recording

### Plugin
- [ ] Button press triggers recording
- [ ] Button release stops + transcribes
- [ ] Visual state changes
- [ ] Alert if daemon offline
- [ ] Health check on load
- [ ] Works with other plugins

### Integration
- [ ] Hotkey + Stream Deck no conflict
- [ ] Text insertion from both sources
- [ ] Config changes apply
- [ ] Error logs show source

---

## Known Limitations

1. **Requires Elgato app running**
2. **Localhost only** (security)
3. **No button release guarantee** (timeout needed)
4. **Single daemon instance** (port conflict if multiple users)

---

## Future Enhancements

- [ ] Multiple language buttons (EN/PL on different keys)
- [ ] Property inspector (config UI)
- [ ] Transcription preview in button
- [ ] Waveform visualization
- [ ] Plugin marketplace submission
- [ ] Stream Deck Mobile/Pedal support

---

## Resources

- [Stream Deck SDK](https://docs.elgato.com/streamdeck/sdk/introduction/getting-started/)
- [Plugin API Reference](https://docs.elgato.com/streamdeck/sdk/references/websocket/plugin/)
- [JavaScript SDK](https://github.com/elgatosf/streamdeck-javascript-sdk)
- [Plugin Samples](https://github.com/elgatosf/streamdeck-plugin-samples)

---

## Unresolved Questions

1. **Port conflicts?** Multiple daemon instances?
   - Config-based port or auto-increment (7777, 7778...)
2. **Plugin updates?** Auto-update or manual GitHub releases?
   - Start: Manual, consider marketplace later
3. **Icon customization?** User-provided images?
   - Phase 2: Property inspector
4. **Simultaneous triggers?** Hotkey during Stream Deck recording?
   - Ignore duplicate starts (409), first trigger wins
