// Jiggler core: emits a synthetic mouse-move event via SendInput.
//
// Two modes:
//   Zen     — dx=0, dy=0. Resets idle timer without moving the cursor at all.
//   Visible — alternates dx=+1 / -1 each tick. Net cursor displacement is zero
//             over any pair of ticks, so the cursor doesn't drift.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Zen,
    Visible,
}

impl Mode {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "visible" => Mode::Visible,
            _ => Mode::Zen,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Zen => "zen",
            Mode::Visible => "visible",
        }
    }
}

pub struct Jiggler {
    pub mode: Mode,
    direction: i32, // +1 or -1, flipped each Visible tick
}

impl Jiggler {
    pub fn new(mode: Mode) -> Self {
        Self { mode, direction: 1 }
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.mode = m;
    }

    /// Inject a single mouse-move event. Failures (e.g. UIPI / locked desktop)
    /// are silently swallowed — there is nothing actionable.
    pub fn tick(&mut self) {
        let (dx, dy) = match self.mode {
            Mode::Zen => (0, 0),
            Mode::Visible => {
                let dx = self.direction;
                self.direction = -self.direction;
                (dx, 0)
            }
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
}
