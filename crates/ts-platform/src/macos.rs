/// `kCGEventSourceStateHIDSystemState`, de `CGEventSource.h`.
///
/// Consulta a camada HID (Human Interface Device): teclado, mouse e trackpad
/// físicos. A alternativa, `kCGEventSourceStateCombinedSessionState` (0),
/// incluiria eventos sintéticos — scripts de automação e "jigglers" que mexem
/// o cursor sozinhos manteriam o advogado como ativo de cadeira vazia.
const HID_SYSTEM_STATE: i32 = 1;
/// `kCGAnyInputEventType`, de `CGEventTypes.h`.
///
/// Não é um tipo de evento: é um curinga com todos os bits ligados, valor que
/// nunca colide com um evento real. Conta qualquer input — inclusive rolagem e
/// movimento de mouse, e não só teclas. Sem isso, ler uma petição rolando a
/// página contaria como ausência.
const ANY_INPUT_EVENT: u32 = 0xFFFFFFFF;

use std::time::Instant;

use objc2_app_kit::NSWorkspace;
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}

pub struct MacOsMonitor {
    start: Instant,
}

pub struct MacOsAppInfo {
    pub name: String,
    pub bundle_id: String,
}

impl MacOsMonitor {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn idle_ms(&self) -> u64 {
        let idle_seconds =
            unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT) };
        (idle_seconds * 1000.0) as u64
    }

    pub fn mono_ms(&self) -> u64 {
        self.start
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }

    pub fn frontmost_app(&mut self) -> Option<MacOsAppInfo> {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let app_name = app.localizedName()?.to_string();
        let app_bundle_id = app.bundleIdentifier()?.to_string();
        Some(MacOsAppInfo {
            name: app_name,
            bundle_id: app_bundle_id,
        })
    }
}
