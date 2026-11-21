//! The Snipping Tool chord types and the pure chord state machine (`ChordFilter`), shared
//! by every backend. The rules it keeps are listed in `snip.rs`.

/// A Snipping Tool shortcut the hook took over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnipChord {
    /// Win+Shift+S: Snipping Tool region snip.
    Region,
    /// Win+Shift+R: Snipping Tool screen recording.
    Record,
}

impl SnipChord {
    pub const ALL: [SnipChord; 2] = [SnipChord::Region, SnipChord::Record];

    /// Display form of the chord, e.g. "Win+Shift+S".
    pub fn display(self) -> &'static str {
        match self {
            SnipChord::Region => "Win+Shift+S",
            SnipChord::Record => "Win+Shift+R",
        }
    }

    /// The same chord in the `global-hotkey` string format, so the app can tell when one
    /// of its own bindings is the chord the hook already handles.
    pub fn hotkey_string(self) -> &'static str {
        match self {
            SnipChord::Region => "Super+Shift+KeyS",
            SnipChord::Record => "Super+Shift+KeyR",
        }
    }

    fn vk(self) -> u32 {
        match self {
            SnipChord::Region => VK_S,
            SnipChord::Record => VK_R,
        }
    }

    pub(crate) fn from_vk(vk: u32) -> Option<SnipChord> {
        SnipChord::ALL.into_iter().find(|c| c.vk() == vk)
    }

    pub(crate) fn index(self) -> usize {
        match self {
            SnipChord::Region => 0,
            SnipChord::Record => 1,
        }
    }
}

const VK_S: u32 = 0x53;
const VK_R: u32 = 0x52;
pub(crate) const VK_SHIFT: u32 = 0x10;
pub(crate) const VK_CONTROL: u32 = 0x11;
pub(crate) const VK_MENU: u32 = 0x12;
pub(crate) const VK_LWIN: u32 = 0x5B;
pub(crate) const VK_RWIN: u32 = 0x5C;
const VK_LSHIFT: u32 = 0xA0;
const VK_RSHIFT: u32 = 0xA1;
const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;
/// Unassigned virtual key injected to keep Win's release from opening Start.
pub const MASK_VK: u32 = 0xE8;
/// `dwExtraInfo` tag on everything this hook injects ("SNIP").
pub const SNIP_EXTRA_INFO: usize = 0x534E_4950;
/// Longest gap between a swallowed letter event and its next auto-repeat. The longest
/// keyboard repeat delay Windows offers is 1 s; repeats after that come every 33-400 ms.
pub const REPEAT_WINDOW_MS: u32 = 1_100;

/// Modifier state, as tracked from events or read from `GetAsyncKeyState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub win: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// One keyboard event as the hook sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub vk: u32,
    pub down: bool,
    /// Event time in milliseconds (`KBDLLHOOKSTRUCT::time`, wraps around).
    pub time: u32,
}

impl KeyEvent {
    pub fn down(vk: u32) -> KeyEvent {
        KeyEvent {
            vk,
            down: true,
            time: 0,
        }
    }
    pub fn up(vk: u32) -> KeyEvent {
        KeyEvent {
            vk,
            down: false,
            time: 0,
        }
    }
    /// The same event at time `ms`.
    pub fn at(self, ms: u32) -> KeyEvent {
        KeyEvent { time: ms, ..self }
    }
}

/// What to do with one event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    /// Stop the event here (return 1 from the hook without calling the next hook).
    pub swallow: bool,
    /// A chord completed: route it to the app.
    pub fire: Option<SnipChord>,
    /// Inject the mask key now so the pending Win release does not open Start.
    pub mask_win: bool,
}

/// The pure chord state machine behind the hook. Feed it every keyboard event in order.
#[derive(Clone, Debug, Default)]
pub struct ChordFilter {
    lwin: bool,
    rwin: bool,
    lshift: bool,
    rshift: bool,
    lctrl: bool,
    rctrl: bool,
    lalt: bool,
    ralt: bool,
    /// Letter keys whose down was swallowed and whose up has not arrived yet.
    swallowed: [bool; 2],
    /// Time of the last swallowed event for each letter (repeat detection).
    last: [u32; 2],
    /// Swallow and report no new chord (see `set_pass_through`).
    pass_through: bool,
}

impl ChordFilter {
    pub fn new() -> ChordFilter {
        ChordFilter::default()
    }

    /// Modifier state as tracked from the events seen so far.
    pub fn mods(&self) -> Mods {
        Mods {
            win: self.lwin || self.rwin,
            shift: self.lshift || self.rshift,
            ctrl: self.lctrl || self.rctrl,
            alt: self.lalt || self.ralt,
        }
    }

    /// Whether a chord letter is held after its down was swallowed.
    pub fn holding(&self) -> bool {
        self.swallowed.iter().any(|s| *s)
    }

    /// While on, no new chord is swallowed or reported: a letter down passes through even
    /// with Win+Shift held. Ups and auto-repeats of a letter already swallowed stay
    /// swallowed, so no lone half of a key press leaks out.
    pub fn set_pass_through(&mut self, on: bool) {
        self.pass_through = on;
    }

    /// Correct the tracked modifiers with the real key state. A modifier that is up for
    /// real is cleared on both sides; one that is down for real but was never seen going
    /// down is attributed to its left key.
    pub fn sync(&mut self, actual: Mods) {
        fn fix(l: &mut bool, r: &mut bool, actual: bool) {
            if !actual {
                *l = false;
                *r = false;
            } else if !*l && !*r {
                *l = true;
            }
        }
        fix(&mut self.lwin, &mut self.rwin, actual.win);
        fix(&mut self.lshift, &mut self.rshift, actual.shift);
        fix(&mut self.lctrl, &mut self.rctrl, actual.ctrl);
        fix(&mut self.lalt, &mut self.ralt, actual.alt);
    }

    /// Process one event. Modifiers are tracked and always pass through; only a chord
    /// letter can be swallowed.
    pub fn on_key(&mut self, ev: KeyEvent) -> Verdict {
        let pass = Verdict::default();
        let slot = match ev.vk {
            VK_LWIN => Some(&mut self.lwin),
            VK_RWIN => Some(&mut self.rwin),
            VK_LSHIFT | VK_SHIFT => Some(&mut self.lshift),
            VK_RSHIFT => Some(&mut self.rshift),
            VK_LCONTROL | VK_CONTROL => Some(&mut self.lctrl),
            VK_RCONTROL => Some(&mut self.rctrl),
            VK_LMENU | VK_MENU => Some(&mut self.lalt),
            VK_RMENU => Some(&mut self.ralt),
            _ => None,
        };
        if let Some(slot) = slot {
            *slot = ev.down;
            if !ev.down && matches!(ev.vk, VK_SHIFT | VK_CONTROL | VK_MENU) {
                // A generic modifier up (injected input) releases both sides.
                match ev.vk {
                    VK_SHIFT => self.rshift = false,
                    VK_CONTROL => self.rctrl = false,
                    _ => self.ralt = false,
                }
            }
            return pass;
        }
        let Some(chord) = SnipChord::from_vk(ev.vk) else {
            return pass;
        };
        let idx = chord.index();
        if !ev.down {
            // The up of a swallowed down is swallowed too, whatever the modifiers are now.
            let swallow = std::mem::replace(&mut self.swallowed[idx], false);
            return Verdict { swallow, ..pass };
        }
        if self.swallowed[idx] {
            if ev.time.wrapping_sub(self.last[idx]) <= REPEAT_WINDOW_MS {
                // Auto-repeat of a letter whose first down was swallowed.
                self.last[idx] = ev.time;
                return Verdict {
                    swallow: true,
                    ..pass
                };
            }
            // Too long since the last swallowed event to be a repeat: the up was missed.
            // Judge this down as a fresh press.
            self.swallowed[idx] = false;
        }
        if self.pass_through {
            return pass;
        }
        let m = Mods {
            win: self.lwin || self.rwin,
            shift: self.lshift || self.rshift,
            ctrl: self.lctrl || self.rctrl,
            alt: self.lalt || self.ralt,
        };
        if m.win && m.shift && !m.ctrl && !m.alt {
            self.swallowed[idx] = true;
            self.last[idx] = ev.time;
            return Verdict {
                swallow: true,
                fire: Some(chord),
                mask_win: true,
            };
        }
        pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: u32 = 0x41;

    fn run(f: &mut ChordFilter, evs: &[KeyEvent]) -> Vec<Verdict> {
        evs.iter().map(|e| f.on_key(*e)).collect()
    }

    fn fired(vs: &[Verdict]) -> Vec<SnipChord> {
        vs.iter().filter_map(|v| v.fire).collect()
    }

    fn swallowed(evs: &[KeyEvent], vs: &[Verdict]) -> Vec<KeyEvent> {
        evs.iter()
            .zip(vs)
            .filter(|(_, v)| v.swallow)
            .map(|(e, _)| *e)
            .collect()
    }

    use KeyEvent as K;

    #[test]
    fn win_shift_s_fires_region_and_swallows_only_s() {
        let mut f = ChordFilter::new();
        let evs = [
            K::down(VK_LWIN),
            K::down(VK_LSHIFT),
            K::down(VK_S),
            K::up(VK_S),
            K::up(VK_LSHIFT),
            K::up(VK_LWIN),
        ];
        let vs = run(&mut f, &evs);
        assert_eq!(fired(&vs), vec![SnipChord::Region]);
        assert_eq!(swallowed(&evs, &vs), vec![K::down(VK_S), K::up(VK_S)]);
        assert!(vs[2].mask_win);
        assert_eq!(vs.iter().filter(|v| v.mask_win).count(), 1);
        assert!(!f.holding());
        assert_eq!(f.mods(), Mods::default());
    }

    #[test]
    fn win_shift_r_fires_record() {
        let mut f = ChordFilter::new();
        let evs = [
            K::down(VK_RWIN),
            K::down(VK_RSHIFT),
            K::down(VK_R),
            K::up(VK_R),
            K::up(VK_RSHIFT),
            K::up(VK_RWIN),
        ];
        let vs = run(&mut f, &evs);
        assert_eq!(fired(&vs), vec![SnipChord::Record]);
        assert_eq!(swallowed(&evs, &vs), vec![K::down(VK_R), K::up(VK_R)]);
    }

    #[test]
    fn modifier_press_order_does_not_matter() {
        let mut f = ChordFilter::new();
        let evs = [K::down(VK_LSHIFT), K::down(VK_LWIN), K::down(VK_S)];
        assert_eq!(fired(&run(&mut f, &evs)), vec![SnipChord::Region]);
    }

    #[test]
    fn release_order_is_safe_and_up_is_swallowed_after_modifiers_go() {
        // Win and Shift released before S: S up must still be swallowed (its down was), and
        // the modifiers' ups must pass through.
        let mut f = ChordFilter::new();
        let evs = [
            K::down(VK_LWIN),
            K::down(VK_LSHIFT),
            K::down(VK_S),
            K::up(VK_LWIN),
            K::up(VK_LSHIFT),
            K::up(VK_S),
        ];
        let vs = run(&mut f, &evs);
        assert_eq!(swallowed(&evs, &vs), vec![K::down(VK_S), K::up(VK_S)]);
        assert!(!f.holding());
        // Afterwards a plain s is typed normally.
        assert_eq!(f.on_key(K::down(VK_S)), Verdict::default());
        assert_eq!(f.on_key(K::up(VK_S)), Verdict::default());
    }

    #[test]
    fn auto_repeat_is_swallowed_and_fires_once() {
        let mut f = ChordFilter::new();
        let mut evs = vec![K::down(VK_LWIN), K::down(VK_LSHIFT)];
        evs.extend(std::iter::repeat_n(K::down(VK_S), 5));
        evs.push(K::up(VK_S));
        let vs = run(&mut f, &evs);
        assert_eq!(fired(&vs), vec![SnipChord::Region]);
        assert_eq!(swallowed(&evs, &vs).len(), 6);
    }

    #[test]
    fn auto_repeat_after_win_release_stays_swallowed() {
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_S)],
        );
        f.on_key(K::up(VK_LWIN));
        f.on_key(K::up(VK_LSHIFT));
        let v = f.on_key(K::down(VK_S));
        assert!(v.swallow && v.fire.is_none());
        assert!(f.on_key(K::up(VK_S)).swallow);
    }

    #[test]
    fn repeated_chords_while_modifiers_stay_down() {
        let mut f = ChordFilter::new();
        let evs = [
            K::down(VK_LWIN),
            K::down(VK_LSHIFT),
            K::down(VK_S),
            K::up(VK_S),
            K::down(VK_R),
            K::up(VK_R),
            K::down(VK_S),
            K::up(VK_S),
        ];
        let vs = run(&mut f, &evs);
        assert_eq!(
            fired(&vs),
            vec![SnipChord::Region, SnipChord::Record, SnipChord::Region]
        );
    }

    #[test]
    fn nothing_else_is_swallowed() {
        let combos: &[&[u32]] = &[
            &[],
            &[VK_LSHIFT],
            &[VK_LWIN],
            &[VK_LCONTROL, VK_LSHIFT],
            &[VK_LWIN, VK_LCONTROL],
            &[VK_LWIN, VK_LSHIFT, VK_LCONTROL],
            &[VK_LWIN, VK_LSHIFT, VK_LMENU],
            &[VK_LWIN, VK_RSHIFT, VK_RCONTROL],
            &[VK_LWIN, VK_LSHIFT, VK_RMENU],
        ];
        for mods in combos {
            for key in [VK_S, VK_R, KEY_A, 0x2C, 0x0D] {
                let mut f = ChordFilter::new();
                let mut evs: Vec<KeyEvent> = mods.iter().map(|m| K::down(*m)).collect();
                evs.push(K::down(key));
                evs.push(K::up(key));
                evs.extend(mods.iter().rev().map(|m| K::up(*m)));
                let vs = run(&mut f, &evs);
                let chord = matches!(mods, [VK_LWIN, VK_LSHIFT] | [VK_LWIN, VK_RSHIFT]);
                assert!(
                    swallowed(&evs, &vs).is_empty() || chord,
                    "mods {mods:x?} key {key:x}"
                );
                assert!(fired(&vs).is_empty(), "mods {mods:x?} key {key:x}");
            }
        }
        // Win+Shift with any other letter passes.
        let mut f = ChordFilter::new();
        let evs = [
            K::down(VK_LWIN),
            K::down(VK_LSHIFT),
            K::down(KEY_A),
            K::up(KEY_A),
        ];
        let vs = run(&mut f, &evs);
        assert!(swallowed(&evs, &vs).is_empty());
    }

    #[test]
    fn modifiers_are_never_swallowed() {
        let mut f = ChordFilter::new();
        for vk in [
            VK_LWIN,
            VK_RWIN,
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
        ] {
            assert_eq!(f.on_key(K::down(vk)), Verdict::default());
        }
        // Even while a chord letter is held.
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_S)],
        );
        assert!(!f.on_key(K::up(VK_LSHIFT)).swallow);
        assert!(!f.on_key(K::up(VK_LWIN)).swallow);
    }

    #[test]
    fn left_and_right_modifiers_are_tracked_separately() {
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_RWIN), K::down(VK_LSHIFT)],
        );
        // Releasing one Win key leaves the other down.
        f.on_key(K::up(VK_LWIN));
        assert!(f.on_key(K::down(VK_S)).fire.is_some());
        f.on_key(K::up(VK_S));
        f.on_key(K::up(VK_RWIN));
        assert!(f.on_key(K::down(VK_S)).fire.is_none());
    }

    #[test]
    fn generic_shift_from_injected_input_counts() {
        let mut f = ChordFilter::new();
        run(&mut f, &[K::down(VK_LWIN), K::down(VK_SHIFT)]);
        assert!(f.on_key(K::down(VK_S)).fire.is_some());
        f.on_key(K::up(VK_S));
        f.on_key(K::up(VK_SHIFT));
        assert!(!f.mods().shift);
    }

    #[test]
    fn stale_win_from_a_missed_key_up_is_corrected_by_sync() {
        // Win+L locks the session: the hook sees Win go down but never sees it come up.
        let mut f = ChordFilter::new();
        run(&mut f, &[K::down(VK_LWIN), K::down(0x4C), K::up(0x4C)]);
        assert!(f.mods().win);
        // Back at the desktop, the user types a capital S.
        f.on_key(K::down(VK_LSHIFT));
        f.sync(Mods {
            shift: true,
            ..Mods::default()
        });
        let v = f.on_key(K::down(VK_S));
        assert_eq!(v, Verdict::default(), "plain Shift+S must reach the app");
        assert!(!f.on_key(K::up(VK_S)).swallow);
    }

    #[test]
    fn sync_picks_up_modifiers_pressed_before_install() {
        let mut f = ChordFilter::new();
        f.sync(Mods {
            win: true,
            shift: true,
            ..Mods::default()
        });
        assert!(f.on_key(K::down(VK_S)).fire.is_some());
        // The sync attributed them to the left keys, whose ups clear them.
        f.on_key(K::up(VK_S));
        f.on_key(K::up(VK_LWIN));
        f.on_key(K::up(VK_LSHIFT));
        assert_eq!(f.mods(), Mods::default());
    }

    #[test]
    fn sync_keeps_the_side_that_was_seen() {
        let mut f = ChordFilter::new();
        f.on_key(K::down(VK_RWIN));
        f.sync(Mods {
            win: true,
            ..Mods::default()
        });
        f.on_key(K::up(VK_RWIN));
        assert!(!f.mods().win);
    }

    #[test]
    fn swallowed_letter_up_survives_sync() {
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_R)],
        );
        f.sync(Mods::default());
        assert!(f.on_key(K::up(VK_R)).swallow);
        assert!(!f.holding());
    }

    #[test]
    fn stray_letter_up_passes() {
        // An S up whose down happened before the hook existed is not ours to swallow.
        let mut f = ChordFilter::new();
        run(&mut f, &[K::down(VK_LWIN), K::down(VK_LSHIFT)]);
        assert!(!f.on_key(K::up(VK_S)).swallow);
    }

    #[test]
    fn missed_letter_up_does_not_leave_the_letter_stuck() {
        // Win+Shift+S, then the secure desktop takes the S up (Ctrl+Alt+Del, UAC, Win+L).
        let mut f = ChordFilter::new();
        let v = run(
            &mut f,
            &[
                K::down(VK_LWIN).at(1_000),
                K::down(VK_LSHIFT).at(1_010),
                K::down(VK_S).at(1_020),
            ],
        );
        assert_eq!(fired(&v), vec![SnipChord::Region]);
        // Repeats inside the window are still swallowed, and extend it.
        assert!(f.on_key(K::down(VK_S).at(1_600)).swallow);
        assert!(f.on_key(K::down(VK_S).at(2_650)).swallow);
        f.on_key(K::up(VK_LSHIFT).at(2_700));
        f.on_key(K::up(VK_LWIN).at(2_710));
        f.sync(Mods::default());
        // Much later a plain s is typed: it reaches the app.
        let late = 2_650 + REPEAT_WINDOW_MS + 1;
        assert_eq!(f.on_key(K::down(VK_S).at(late)), Verdict::default());
        assert!(!f.on_key(K::up(VK_S).at(late + 50)).swallow);
        // And a new Win+Shift+S fires again.
        f.on_key(K::down(VK_LWIN).at(late + 100));
        f.on_key(K::down(VK_LSHIFT).at(late + 110));
        assert_eq!(
            f.on_key(K::down(VK_S).at(late + 120)).fire,
            Some(SnipChord::Region)
        );
    }

    #[test]
    fn missed_letter_up_then_chord_fires_after_the_window() {
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_S).at(5)],
        );
        // Win+Shift still down, no S up seen: the next press 1.2 s later fires.
        let v = f.on_key(K::down(VK_S).at(5 + 1_200));
        assert_eq!(v.fire, Some(SnipChord::Region));
        assert!(v.swallow);
    }

    #[test]
    fn repeat_window_survives_tick_wraparound() {
        let mut f = ChordFilter::new();
        let t0 = u32::MAX - 100;
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_S).at(t0)],
        );
        let v = f.on_key(K::down(VK_S).at(t0.wrapping_add(500)));
        assert!(v.swallow && v.fire.is_none());
    }

    #[test]
    fn pass_through_lets_chords_reach_windows() {
        let mut f = ChordFilter::new();
        f.set_pass_through(true);
        let evs = [
            K::down(VK_LWIN),
            K::down(VK_LSHIFT),
            K::down(VK_S),
            K::up(VK_S),
            K::down(VK_R),
            K::up(VK_R),
        ];
        let vs = run(&mut f, &evs);
        assert!(fired(&vs).is_empty());
        assert!(swallowed(&evs, &vs).is_empty());
        assert!(vs.iter().all(|v| !v.mask_win));
        // Modifiers were still tracked: turning it off makes the next press a chord.
        f.set_pass_through(false);
        assert_eq!(f.on_key(K::down(VK_S)).fire, Some(SnipChord::Region));
    }

    #[test]
    fn pass_through_keeps_swallowing_a_letter_already_held() {
        let mut f = ChordFilter::new();
        run(
            &mut f,
            &[K::down(VK_LWIN), K::down(VK_LSHIFT), K::down(VK_S)],
        );
        f.set_pass_through(true);
        let v = f.on_key(K::down(VK_S));
        assert!(v.swallow && v.fire.is_none());
        assert!(f.on_key(K::up(VK_S)).swallow);
        assert!(!f.holding());
    }

    #[test]
    fn chord_strings() {
        assert_eq!(SnipChord::Region.display(), "Win+Shift+S");
        assert_eq!(SnipChord::Record.display(), "Win+Shift+R");
        for c in SnipChord::ALL {
            assert_eq!(SnipChord::ALL[c.index()], c);
            assert_eq!(SnipChord::from_vk(c.vk()), Some(c));
        }
    }
}
