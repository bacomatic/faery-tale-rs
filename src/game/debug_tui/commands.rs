//! Debug TUI command dispatch: `execute_command`, individual `/cmd` handlers,
//! and the small set of pure helpers used by both the dispatcher and the
//! renderer (log filtering + modal formatting).
//!
//! Feature-gated behind `debug-tui`; compiled together with `view.rs`.

use super::bridge::*;
use super::view::DebugConsole;

impl DebugConsole {
    pub(super) fn execute_command(&mut self, raw: &str) {
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.is_empty() { return; }
        let cmd = parts[0].to_ascii_lowercase();
        let args = &parts[1..];

        match cmd.as_str() {
            "/help" | "/h" | "/?" => self.cmd_help(args),
            "/kill" => self.cmd_kill(args),
            "/die" => {
                self.push_cmd(DebugCommand::AdjustStat { stat: StatId::Vitality, delta: -9999 });
                self.log("Player vitality set to zero.");
            }
            "/pack" => self.push_cmd(DebugCommand::HeroPack),
            "/max" => self.cmd_max_stats(),
            "/heal" => self.cmd_heal(),
            "/stat" => self.cmd_stat(args),
            "/stats" => self.cmd_stats(),
            "/quest" => self.cmd_quest(),
            "/inventory" | "/inventorylist" => self.cmd_inventory(),
            "/inv" => self.cmd_inv(args),
            "/give" => self.cmd_give(args),
            "/take" => self.cmd_take(args),
            "/cheat" => self.cmd_cheat(args),
            "/tp" | "/teleport" => self.cmd_tp(args),
            "/god" => self.cmd_god(args),
            "/noclip" => self.cmd_god(&["noclip"]),
            "/magic" => self.cmd_magic(args),
            "/swan" => self.push_cmd(DebugCommand::SummonSwan),
            "/time" => self.cmd_time(args),
            "/brother" => self.cmd_brother(args),
            "/fx" => self.cmd_fx(args),
            "/actors" => self.push_cmd(DebugCommand::QueryActors),
            "/terrain" => self.push_cmd(DebugCommand::QueryTerrain),
            "/doors" => self.push_cmd(DebugCommand::QueryDoors),
            "/extent" => self.push_cmd(DebugCommand::QueryExtent),
            "/encounter" => self.cmd_encounter(args),
            "/items" => self.cmd_items(args),
            "/songs" => self.cmd_songs(args),
            "/adf" => self.cmd_adf(args),
            "/clear" | "/cls" => self.cmd_clear(),
            "/pause" => {
                self.pause_request = Some(true);
                self.log("Game paused. /resume to continue, /step [n] to advance frames.");
            }
            "/resume" | "/unpause" => {
                self.pause_request = Some(false);
                self.log("Game resumed.");
            }
            "/step" => {
                let n: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                let n = n.max(1);
                self.step_request = self.step_request.saturating_add(n);
                self.pause_request = Some(true);
                self.log(format!("Stepping {} tick(s).", n));
            }
            "/watch" => {
                self.watch_expanded = !self.watch_expanded;
                self.log(format!(
                    "Actor watch {}.",
                    if self.watch_expanded { "expanded" } else { "collapsed" }
                ));
            }
            "/filter" => self.cmd_filter(args),
            _ => {
                self.log(format!("Unknown command: {}  (type /help for list)", cmd));
            }
        }
    }

    pub(super) fn push_cmd(&mut self, cmd: DebugCommand) {
        self.pending_commands.push(cmd);
    }

    // ── Individual commands ───────────────────────────────────────────────────

    fn cmd_help(&mut self, args: &[&str]) {
        if let Some(&topic) = args.first() {
            let msg = match topic.to_ascii_lowercase().as_str() {
                "/kill" | "kill"     => "/kill — kill all hostile enemies on screen.\n  /kill <slot>  kill one actor slot (1-19).",
                "/die"  | "die"      => "/die — set player vitality to zero (die).",
                "/pack" | "pack"     => "/pack — fill weapons, magic items, keys, and arrows.",
                "/max"  | "max"      => "/max — set all stats to maximum / hunger+fatigue to 0.",
                "/heal" | "heal"     => "/heal — vitality to 15 + brave/4, hunger=0, fatigue=0.",
                "/stat" | "stat"     => "/stat <name> [+|-]<value>  e.g. /stat vit 100 or /stat hunger -50\n  Names: vit, brv, lck, knd, wlt, hgr, ftg",
                "/stats"            => "/stats — full hero stat dump to log.",
                "/quest"            => "/quest — quest progress (princess, statues, writ, talisman).",
                "/inventory"        => "/inventory — full stuff[] dump grouped by category.",
                "/inv"  | "inv"      => "/inv <slot 0-34> [+|-]<value>  e.g. /inv 0 1 or /inv 8 +99",
                "/give" | "give"     => "/give <item>  add 1 x item by name or stuff index (see /items).",
                "/take" | "take"     => "/take <item>  remove 1 x item by name or stuff index.",
                "/cheat"| "cheat"    => "/cheat          toggle cheat1 debug-key mode\n  /cheat on|off  set explicitly.",
                "/tp"   | "teleport" => "/tp safe | ring <N> | <x> <y> | <location>\n  e.g. /tp 200 150   /tp tavern   /tp ring 0",
                "/god"  | "god"      => "/god [noclip|invincible|ohk|reach|all|off]  — toggle god mode flag.",
                "/noclip"           => "/noclip — shortcut for /god noclip.",
                "/magic"| "magic"    => "/magic <light|secret|freeze> — toggle sticky magic effect.",
                "/swan" | "swan"     => "/swan — summon the swan.",
                "/time" | "time"     => "/time <HH:MM> | dawn | noon | dusk | midnight | hold | resume\n  /time hold — freeze time.  /time resume — unfreeze.",
                "/brother"          => "/brother <julian|phillip|kevin>",
                "/fx"   | "fx"      => "/fx <witch|teleport|fadeout|fadein>",
                "/actors"           => "/actors — print actor list to log.",
                "/terrain"          => "/terrain — dump terra lookup chain at hero's feet (collision debug).",
                "/doors"            => "/doors — list doors in current region + key inventory.",
                "/extent"           => "/extent — dump extent zone under the hero's feet.",
                "/encounter"        => "/encounter — force regional encounter (4 enemies).\n  /encounter <type>  spawn one enemy: orc ghost skeleton wraith dragon snake swan horse\n  /encounter clear   deactivate all active NPCs",
                "/items"            => "/items — scatter items around player.\n  /items             all 30 safe items\n  /items <count>     N random items (no talisman)\n  /items <name|id>   drop one item by name or index 0-30\n  /items <n> <name>  drop N of a named item\n  Note: talisman (triggers end-of-game) only drops with: /items talisman",
                "/songs"| "songs"   => "/songs — list song groups.  /songs play <N>  /songs stop  /songs cave <on|off>",
                "/adf"  | "adf"     => "/adf <block> [count] — hex dump ADF block(s) to log.",
                "/clear"| "cls"     => "/clear — clear the log.",
                "/filter"|"filter"  => "/filter — open interactive category toggle (Up/Down or Tab to move, Space to toggle, Enter/Esc to close).\n  /filter all     enable every category.\n  /filter none    disable every category.\n  /filter reset   defaults (noisy categories off).\n  /filter +CAT -CAT  toggle by name (combat, movement, ai, ...).",
                "/watch"| "watch"   => "/watch — toggle the actor watch panel between collapsed and expanded (same as Ctrl+W).",
                "/pause"| "pause"   => "/pause — freeze the game loop (actors + physics). Daynight still ticks unless /time hold. Shortcut: Ctrl+P.",
                "/resume"|"resume"  => "/resume — unfreeze the game loop (alias: /unpause). Shortcut: Ctrl+P.",
                "/step" | "step"    => "/step [n] — while paused, advance exactly 1 (or n) frame(s).",
                _ => "No help for that topic.",
            };
            for line in msg.lines() {
                self.log(line);
            }
            return;
        }

        let lines = [
            "— Commands ———————————————————————————",
            "  /kill          kill enemies on screen  (/kill <slot> = one actor)",
            "  /die           kill the player",
            "  /pack          fill weapons, magic, keys",
            "  /max           max all stats",
            "  /heal          heal vitality + clear hunger/fatigue",
            "  /stat <n> <v>  set/adjust a stat (vit/brv/lck/knd/wlt/hgr/ftg)",
            "  /stats         dump all hero stats",
            "  /quest         dump quest progress",
            "  /inventory     dump full stuff[] array",
            "  /inv <s> <v>   set/adjust inventory slot 0-34",
            "  /give <item>   add 1 x item (name or index)",
            "  /take <item>   remove 1 x item (name or index)",
            "  /cheat [on|off] toggle / set cheat1 mode",
            "  /tp <x> <y>    teleport (also: /tp safe | ring <N> | <location>)",
            "  /god [flag]    god mode: noclip/invincible/ohk/reach/all/off",
            "  /noclip        toggle noclip shortcut",
            "  /magic <m>     sticky magic: light/secret/freeze",
            "  /swan          summon the swan",
            "  /time <t>      set time: HH:MM or dawn/noon/dusk/midnight/hold/resume",
            "  /brother <b>   switch to julian/phillip/kevin",
            "  /fx <e>        trigger: witch/teleport/fadeout/fadein",
            "  /actors        list actors",
            "  /terrain       dump terrain at current position",
            "  /doors         dump door + key state",
            "  /extent        dump extent zone under the hero",
            "  /encounter [t] force encounter / spawn type / clear",
            "  /items [n] [name]  scatter items around player (no talisman unless named)",
            "  /songs [cmd]   music: play <N> / stop / cave <on|off>",
            "  /adf <b> [n]   hex dump n ADF block(s) starting at b",
            "  /clear         clear this log",
            "  /filter [...]  show/adjust log categories",
            "  /watch         toggle actor watch panel (also: Ctrl+W)",
            "  /pause         freeze game loop (also: Ctrl+P)",
            "  /resume        unfreeze game loop (also: Ctrl+P)",
            "  /step [n]      advance 1 (or n) frame(s) while paused",
            "  /help [cmd]    show help",
            "——————————————————————————————————————",
            "PgUp/PgDn/Home/End — scroll log   Up/Down — command history",
        ];
        for l in &lines { self.log(*l); }
    }

    fn cmd_max_stats(&mut self) {
        use StatId::*;
        for (s, v) in &[
            (Vitality, 999i16), (Brave, 255), (Luck, 255),
            (Kind, 255), (Wealth, 9999), (Hunger, 0), (Fatigue, 0),
        ] {
            self.push_cmd(DebugCommand::SetStat { stat: *s, value: *v });
        }
        self.log("Max stats applied.");
    }

    fn cmd_heal(&mut self) {
        let cap = 15i16.saturating_add((self.status.brave as i16) / 4);
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Vitality, value: cap });
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Hunger, value: 0 });
        self.push_cmd(DebugCommand::SetStat { stat: StatId::Fatigue, value: 0 });
        self.log(format!("Healed: vitality={} (cap 15 + brave/4), hunger=0, fatigue=0.", cap));
    }

    fn cmd_stats(&mut self) {
        let s = &self.status;
        let heal_cap = 15i16.saturating_add((s.brave as i16) / 4);
        let lines = [
            format!("── Hero Stats ──"),
            format!("  Vitality: {} / {}   Brave: {}   Luck: —   Kind: —",
                s.vitality, heal_cap, s.brave),
            format!("  Wealth: {}g   Hunger: {}   Fatigue: {}",
                s.wealth, s.hunger, s.fatigue),
            format!("  Position: ({}, {})   Region: {}   Brother: {}",
                s.hero_x, s.hero_y, s.region_num, s.brother),
            format!("  God mode: {:#06b}   Cheat1: {}   Paused: {}",
                s.god_mode_flags, s.cheat1, s.is_paused),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    fn cmd_quest(&mut self) {
        let s = &self.status;
        let key_count: u16 = s.stuff.iter().skip(16).take(6).map(|&v| v as u16).sum();
        let lines = [
            format!("── Quest Progress ──"),
            format!("  Princess captive: {}   Rescues: {}",
                s.princess_captive, s.princess_rescues),
            format!("  Gold statues: {} / 5   Writ of Safe Conduct: {}",
                s.statues_collected, if s.has_writ { "yes" } else { "no" }),
            format!("  TALISMAN: {}   Keys held (16-21): {}",
                if s.has_talisman { "YES (win condition!)" } else { "no" }, key_count),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    fn cmd_inventory(&mut self) {
        let s = &self.status.stuff;
        if s.is_empty() {
            self.log("Inventory not available (not in gameplay).");
            return;
        }
        let get = |i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
        let lines = [
            format!("── Inventory (stuff[{}]) ──", s.len()),
            format!("  Weapons : dirk={} mace={} sword={} bow={} wand={} lasso={} shell={} [7]={}",
                get(0), get(1), get(2), get(3), get(4), get(5), get(6), get(7)),
            format!("  Arrows  : {}", get(8)),
            format!("  Magic   : vial={} jewel={} totem={} flute={} ring={} skull={} staff={}",
                get(9), get(10), get(11), get(12), get(13), get(14), get(15)),
            format!("  Keys    : gold={} silver={} ruby={} skull={} iron={} crystal={}",
                get(16), get(17), get(18), get(19), get(20), get(21)),
            format!("  Quest   : talisman={} writ={} statues={}",
                get(22), get(28), get(25)),
            format!("  Consume : food={} fruit={}", get(23), get(24)),
            format!("  Gold    : {}", self.status.wealth),
        ];
        for l in &lines { self.log(l.clone()); }
    }

    fn cmd_kill(&mut self, args: &[&str]) {
        if args.is_empty() {
            self.push_cmd(DebugCommand::InstaKill);
        } else {
            match args[0].parse::<u8>() {
                Ok(slot) if slot >= 1 && slot <= 19 => {
                    self.push_cmd(DebugCommand::KillActorSlot { slot });
                }
                Ok(_) => self.log("/kill: slot must be 1-19"),
                Err(_) => self.log(format!("/kill: bad slot '{}'", args[0])),
            }
        }
    }

    fn cmd_give(&mut self, args: &[&str]) {
        let Some(raw) = args.first() else {
            self.log("Usage: /give <item>  (name or stuff index)");
            return;
        };
        match crate::game::debug_items::lookup_by_name(raw)
            .or_else(|| raw.parse::<u8>().ok().and_then(crate::game::debug_items::lookup_by_id))
        {
            Some(entry) => {
                self.push_cmd(DebugCommand::AdjustInventory {
                    index: entry.stuff_index as u8, delta: 1,
                });
                self.log(format!("Gave 1 x {} (stuff[{}]).", entry.name, entry.stuff_index));
            }
            None => self.log(format!("/give: unknown item '{}'", raw)),
        }
    }

    fn cmd_take(&mut self, args: &[&str]) {
        let Some(raw) = args.first() else {
            self.log("Usage: /take <item>  (name or stuff index)");
            return;
        };
        match crate::game::debug_items::lookup_by_name(raw)
            .or_else(|| raw.parse::<u8>().ok().and_then(crate::game::debug_items::lookup_by_id))
        {
            Some(entry) => {
                self.push_cmd(DebugCommand::AdjustInventory {
                    index: entry.stuff_index as u8, delta: -1,
                });
                self.log(format!("Took 1 x {} (stuff[{}]).", entry.name, entry.stuff_index));
            }
            None => self.log(format!("/take: unknown item '{}'", raw)),
        }
    }

    fn cmd_cheat(&mut self, args: &[&str]) {
        let enabled = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("") => !self.status.cheat1,
            Some("on")      => true,
            Some("off")     => false,
            Some(other) => {
                self.log(format!("/cheat: unknown arg '{}' (use on/off or no arg to toggle)", other));
                return;
            }
        };
        self.push_cmd(DebugCommand::SetCheat1 { enabled });
        self.log(format!("cheat1 -> {}", if enabled { "ON" } else { "OFF" }));
    }

    fn cmd_stat(&mut self, args: &[&str]) {
        if args.len() < 2 {
            self.log("Usage: /stat <name> [+|-]<value>  (vit brv lck knd wlt hgr ftg)");
            return;
        }
        let stat = match args[0].to_ascii_lowercase().as_str() {
            "vit" | "vitality"  => StatId::Vitality,
            "brv" | "brave"     => StatId::Brave,
            "lck" | "luck"      => StatId::Luck,
            "knd" | "kind"      => StatId::Kind,
            "wlt" | "wealth"    => StatId::Wealth,
            "hgr" | "hunger"    => StatId::Hunger,
            "ftg" | "fatigue"   => StatId::Fatigue,
            other => {
                self.log(format!("Unknown stat: {}  (use vit brv lck knd wlt hgr ftg)", other));
                return;
            }
        };
        let raw_val = args[1];
        let is_delta = raw_val.starts_with('+') || raw_val.starts_with('-');
        if is_delta {
            match raw_val.parse::<i16>() {
                Ok(delta) => self.push_cmd(DebugCommand::AdjustStat { stat, delta }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        } else {
            match raw_val.parse::<i16>() {
                Ok(val) => self.push_cmd(DebugCommand::SetStat { stat, value: val }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        }
    }

    fn cmd_inv(&mut self, args: &[&str]) {
        if args.len() < 2 {
            self.log("Usage: /inv <slot 0-34> [+|-]<value>");
            return;
        }
        let slot: u8 = match args[0].parse() {
            Ok(s) if s < 35 => s,
            _ => { self.log("Slot must be 0-34."); return; }
        };
        let raw_val = args[1];
        let is_delta = raw_val.starts_with('+') || raw_val.starts_with('-');
        if is_delta {
            match raw_val.parse::<i8>() {
                Ok(delta) => self.push_cmd(DebugCommand::AdjustInventory { index: slot, delta }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        } else {
            match raw_val.parse::<u8>() {
                Ok(val) => self.push_cmd(DebugCommand::SetInventory { index: slot, value: val }),
                Err(_) => self.log(format!("Bad value: {}", raw_val)),
            }
        }
    }

    fn cmd_tp(&mut self, args: &[&str]) {
        match args {
            [] => self.log("Usage: /tp safe | ring <N> | <x> <y> | <location>"),
            ["safe"] | ["Safe"] => self.push_cmd(DebugCommand::TeleportSafe),
            ["ring", n] => {
                match n.parse::<u8>() {
                    Ok(idx) => self.push_cmd(DebugCommand::TeleportStoneRing { index: idx }),
                    Err(_) => self.log(format!("Bad ring index: {}", n)),
                }
            }
            [xs, ys] if xs.chars().next().map_or(false, |c| c.is_ascii_digit())
                     && ys.chars().next().map_or(false, |c| c.is_ascii_digit()) => {
                let x = xs.parse::<u16>();
                let y = ys.parse::<u16>();
                match (x, y) {
                    (Ok(x), Ok(y)) => self.push_cmd(DebugCommand::TeleportCoords { x, y }),
                    _ => self.log("Usage: /tp <x> <y>  (unsigned integers)"),
                }
            }
            _ => {
                let name = args.join(" ");
                self.push_cmd(DebugCommand::TeleportNamedLocation { name: name.clone() });
                self.log(format!("Teleport request: '{}'", name));
            }
        }
    }

    fn cmd_god(&mut self, args: &[&str]) {
        let current = GodModeFlags::from_bits_truncate(self.status.god_mode_flags);
        let new_flags = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("noclip")     => current ^ GodModeFlags::NOCLIP,
            Some("invincible") => current ^ GodModeFlags::INVINCIBLE,
            Some("ohk") | Some("onehit") => current ^ GodModeFlags::ONE_HIT_KILL,
            Some("reach") | Some("insane") => current ^ GodModeFlags::INSANE_REACH,
            Some("all") | Some("on") => GodModeFlags::all(),
            Some("off") | Some("none") => GodModeFlags::empty(),
            None | Some("") => {
                let s = build_god_str(self.status.god_mode_flags);
                self.log(format!("God mode: {}", if s.is_empty() { "off" } else { &s }));
                return;
            }
            Some(other) => {
                self.log(format!("Unknown flag: {}  (noclip/invincible/ohk/reach/all/off)", other));
                return;
            }
        };
        self.push_cmd(DebugCommand::SetGodMode { flags: new_flags });
        let s = build_god_str(new_flags.bits());
        self.log(format!("God mode: {}", if s.is_empty() { "off" } else { &s }));
        self.status.god_mode_flags = new_flags.bits();
    }

    fn cmd_magic(&mut self, args: &[&str]) {
        let effect = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("light")   => MagicEffect::Light,
            Some("secret")  => MagicEffect::Secret,
            Some("freeze")  => MagicEffect::Freeze,
            _ => { self.log("Usage: /magic <light|secret|freeze>"); return; }
        };
        self.push_cmd(DebugCommand::ToggleMagicEffect { effect });
    }

    fn cmd_time(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("hold")      => self.push_cmd(DebugCommand::HoldTimeOfDay { hold: true }),
            Some("resume") | Some("free") | Some("unhold") => self.push_cmd(DebugCommand::HoldTimeOfDay { hold: false }),
            Some("midnight")  => self.push_cmd(DebugCommand::SetDayPhase { phase: 0 }),
            Some("dawn")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 6000 }),
            Some("noon")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 12000 }),
            Some("dusk")      => self.push_cmd(DebugCommand::SetDayPhase { phase: 18000 }),
            Some(hhmm) => {
                let parts: Vec<&str> = hhmm.split(':').collect();
                if parts.len() == 2 {
                    let h = parts[0].parse::<u8>();
                    let m = parts[1].parse::<u8>();
                    match (h, m) {
                        (Ok(hour), Ok(minute)) if hour < 24 && minute < 60 => {
                            self.push_cmd(DebugCommand::SetGameTime { hour, minute });
                        }
                        _ => self.log("Usage: /time HH:MM  (e.g. /time 08:30)"),
                    }
                } else {
                    self.log("Usage: /time <HH:MM | dawn | noon | dusk | midnight | hold | resume>");
                }
            }
            None => {
                self.log(format!(
                    "Game time: day {} {:02}:{:02}  phase={:?}  {}",
                    self.status.game_day,
                    self.status.game_hour,
                    self.status.game_minute,
                    self.status.day_phase,
                    if self.status.time_held { "[HELD]" } else { "" }
                ));
            }
        }
    }

    fn cmd_brother(&mut self, args: &[&str]) {
        let brother = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("julian")  => BrotherId::Julian,
            Some("phillip") => BrotherId::Phillip,
            Some("kevin")   => BrotherId::Kevin,
            _ => { self.log("Usage: /brother <julian|phillip|kevin>"); return; }
        };
        self.push_cmd(DebugCommand::RestartAsBrother { brother });
    }

    fn cmd_fx(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("witch")    => self.push_cmd(DebugCommand::TriggerWitchEffect),
            Some("teleport") => self.push_cmd(DebugCommand::TriggerTeleportEffect),
            Some("fadeout")  => self.push_cmd(DebugCommand::TriggerPaletteTransition { to_black: true }),
            Some("fadein")   => self.push_cmd(DebugCommand::TriggerPaletteTransition { to_black: false }),
            _ => self.log("Usage: /fx <witch|teleport|fadeout|fadein>"),
        }
    }

    fn cmd_songs(&mut self, args: &[&str]) {
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("play") => {
                match args.get(1).and_then(|s| s.parse::<usize>().ok()) {
                    Some(n) if n >= 1 => {
                        self.song_group_requested = Some(n - 1);
                        self.log(format!("Playing song group {}.", n));
                    }
                    _ => self.log("Usage: /songs play <N>  (1-based group number)"),
                }
            }
            Some("stop") => {
                self.stop_requested = true;
                self.log("Music stopped.");
            }
            Some("cave") => {
                match args.get(1).map(|s| s.to_ascii_lowercase()).as_deref() {
                    Some("on") => {
                        self.cave_mode_requested = Some(true);
                        self.log("Cave instrument mode ON (slot 10 → wave=3, vol=7).");
                    }
                    Some("off") => {
                        self.cave_mode_requested = Some(false);
                        self.log("Cave instrument mode OFF (slot 10 → default).");
                    }
                    _ => self.log("Usage: /songs cave <on|off>"),
                }
            }
            _ => {
                let count = self.status.song_group_count;
                if count == 0 {
                    self.log("No songs loaded.");
                } else {
                    self.log(format!("{} song groups available.", count));
                    let cur = self.status.current_song_group;
                    for i in 0..count {
                        let marker = if cur == Some(i) { " ◄ playing" } else { "" };
                        self.log(format!("  /songs play {}  — group {}{}", i + 1, i + 1, marker));
                    }
                    let cave_label = if self.status.cave_mode { "ON" } else { "OFF" };
                    self.log(format!("Cave mode: {}", cave_label));
                    self.log("/songs stop  — stop music");
                    self.log("/songs cave <on|off>  — cave instrument override");
                }
            }
        }
    }

    fn cmd_adf(&mut self, args: &[&str]) {
        let (block, count) = match args {
            [b] => match b.parse::<u32>() {
                Ok(b) => (b, 1u32),
                Err(_) => { self.log("Usage: /adf <block> [count]"); return; }
            },
            [b, c] => match (b.parse::<u32>(), c.parse::<u32>()) {
                (Ok(b), Ok(c)) if c >= 1 => (b, c),
                _ => { self.log("Usage: /adf <block> [count]  (count must be >= 1)"); return; }
            },
            _ => { self.log("Usage: /adf <block> [count]"); return; }
        };
        self.push_cmd(DebugCommand::DumpAdfBlock { block, count });
    }

    fn cmd_encounter(&mut self, args: &[&str]) {
        use crate::game::npc::*;
        match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
            None => self.push_cmd(DebugCommand::SpawnEncounterRandom),
            Some("clear") => self.push_cmd(DebugCommand::ClearEncounters),
            Some(name) => {
                let npc_type = match name {
                    "orc"      => Some(NPC_TYPE_ORC),
                    "ghost"    => Some(NPC_TYPE_GHOST),
                    "skeleton" => Some(NPC_TYPE_SKELETON),
                    "wraith"   => Some(NPC_TYPE_WRAITH),
                    "dragon"   => Some(NPC_TYPE_DRAGON),
                    "snake"    => Some(NPC_TYPE_SKELETON),
                    "swan"     => Some(NPC_TYPE_SWAN),
                    "horse"    => Some(NPC_TYPE_HORSE),
                    _ => None,
                };
                match npc_type {
                    Some(t) => self.push_cmd(DebugCommand::SpawnEncounterType(t)),
                    None => self.log(format!(
                        "Unknown enemy type: {}.  Valid: orc ghost skeleton wraith dragon snake swan horse",
                        name
                    )),
                }
            }
        }
    }

    fn cmd_items(&mut self, args: &[&str]) {
        match args {
            [] => {
                self.push_cmd(DebugCommand::ScatterItems { count: 30, item_id: None });
            }
            [arg] => {
                if let Ok(n) = arg.parse::<usize>() {
                    self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: None });
                } else {
                    match crate::game::sprites::item_name_to_id(arg) {
                        Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: 1, item_id: Some(id) }),
                        None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", arg)),
                    }
                }
            }
            [count_str, name] => {
                match count_str.parse::<usize>() {
                    Err(_) => self.log(format!(
                        "Invalid count '{}'. Usage: /items [count] [name|index]", count_str
                    )),
                    Ok(n) => match crate::game::sprites::item_name_to_id(name) {
                        Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: Some(id) }),
                        None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", name)),
                    },
                }
            }
            _ => self.log("Usage: /items [count] [name|index]  e.g. /items 5 sword".to_string()),
        }
    }

    fn cmd_clear(&mut self) {
        self.log_entries.clear();
        self.scroll_from_bottom = 0;
    }

    /// DBG-LOG-06/07/08: `/filter` command dispatcher.
    fn cmd_filter(&mut self, args: &[&str]) {
        if args.is_empty() {
            self.filter_interactive = Some(0);
            self.log("Filter: interactive mode — Up/Down or Tab to move, Space to toggle, Enter/Esc to close.");
            return;
        }
        if args.len() == 1 {
            match args[0].to_ascii_lowercase().as_str() {
                "all" => {
                    self.active_categories = LogCategory::ALL.iter().copied().collect();
                    self.log("Filter: all categories enabled.");
                    return;
                }
                "none" => {
                    self.active_categories.clear();
                    self.log("Filter: all categories disabled.");
                    return;
                }
                "reset" => {
                    self.active_categories = LogCategory::ALL
                        .iter()
                        .copied()
                        .filter(|c| c.default_enabled())
                        .collect();
                    self.log("Filter: defaults restored (noisy categories off).");
                    return;
                }
                _ => {}
            }
        }
        for tok in args {
            let (sign, rest) = match tok.as_bytes().first() {
                Some(b'+') => (true, &tok[1..]),
                Some(b'-') => (false, &tok[1..]),
                _ => {
                    self.log(format!("Filter: token {:?} missing +/- prefix", tok));
                    continue;
                }
            };
            let Some(cat) = parse_category(rest) else {
                self.log(format!("Filter: unknown category {:?}", rest));
                continue;
            };
            if sign {
                self.active_categories.insert(cat);
                self.log(format!("Filter: +{}", cat.label()));
            } else {
                self.active_categories.remove(&cat);
                self.log(format!("Filter: -{}", cat.label()));
            }
        }
    }
}

// ── Pure helpers shared with view.rs and tests ───────────────────────────────

fn build_god_str(flags: u8) -> String {
    let f = GodModeFlags::from_bits_truncate(flags);
    let mut parts = Vec::new();
    if f.contains(GodModeFlags::NOCLIP)       { parts.push("NOCLIP"); }
    if f.contains(GodModeFlags::INVINCIBLE)   { parts.push("INVINCIBLE"); }
    if f.contains(GodModeFlags::ONE_HIT_KILL) { parts.push("ONE_HIT_KILL"); }
    if f.contains(GodModeFlags::INSANE_REACH) { parts.push("INSANE_REACH"); }
    parts.join("+")
}

/// Parse a category name token (case-insensitive) into a [`LogCategory`].
pub(super) fn parse_category(name: &str) -> Option<LogCategory> {
    let up = name.to_ascii_uppercase();
    LogCategory::ALL.iter().copied().find(|c| c.label() == up.as_str())
}

/// Filter log entries by the given active-category set (DBG-LOG-05).
pub(super) fn filter_log_entries<'a>(
    entries: &'a [DebugLogEntry],
    active: &std::collections::HashSet<LogCategory>,
) -> Vec<&'a DebugLogEntry> {
    entries.iter().filter(|e| active.contains(&e.category)).collect()
}

/// Format a single log entry for rendering.
pub(super) fn format_log_entry(entry: &DebugLogEntry) -> String {
    if entry.timestamp_ticks != 0 {
        format!("[{}] [{}] {}", entry.timestamp_ticks, entry.category.label(), entry.text)
    } else {
        format!("[{}] {}", entry.category.label(), entry.text)
    }
}

/// DBG-LOG-08: render one line per category for the interactive /filter modal.
pub(super) fn filter_modal_lines(
    cursor: usize,
    active: &std::collections::HashSet<LogCategory>,
) -> Vec<String> {
    let mut out = Vec::with_capacity(LogCategory::ALL.len() + 1);
    for (i, cat) in LogCategory::ALL.iter().enumerate() {
        let marker = if i == cursor { ">" } else { " " };
        let state = if active.contains(cat) { "[ON] " } else { "[OFF]" };
        out.push(format!("{} {} {}", marker, state, cat.label()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::view::MAX_LOG_LINES;

    fn make_entry(cat: LogCategory, ticks: u64, text: &str) -> DebugLogEntry {
        DebugLogEntry { category: cat, timestamp_ticks: ticks, text: text.to_owned() }
    }

    /// Mimics `DebugConsole::log_entry` for testing without allocating a
    /// terminal.  Kept in sync with the real implementation above.
    fn push(entries: &mut Vec<DebugLogEntry>, entry: DebugLogEntry) {
        for line in entry.text.split('\n') {
            entries.push(DebugLogEntry {
                category: entry.category,
                timestamp_ticks: entry.timestamp_ticks,
                text: line.to_owned(),
            });
        }
        if entries.len() > MAX_LOG_LINES {
            let overflow = entries.len() - MAX_LOG_LINES;
            entries.drain(..overflow);
        }
    }

    #[test]
    fn log_entry_appends_and_respects_max_lines() {
        let mut entries: Vec<DebugLogEntry> = Vec::new();
        for i in 0..(MAX_LOG_LINES + 25) {
            push(&mut entries, make_entry(LogCategory::General, 0, &format!("msg {}", i)));
        }
        assert_eq!(entries.len(), MAX_LOG_LINES);
        assert_eq!(entries[0].text, "msg 25");
        assert_eq!(entries.last().unwrap().text, format!("msg {}", MAX_LOG_LINES + 24));
    }

    #[test]
    fn log_entry_splits_on_newlines_preserving_category() {
        let mut entries: Vec<DebugLogEntry> = Vec::new();
        push(&mut entries, make_entry(LogCategory::Combat, 42, "line a\nline b\nline c"));
        assert_eq!(entries.len(), 3);
        for e in &entries {
            assert_eq!(e.category, LogCategory::Combat);
            assert_eq!(e.timestamp_ticks, 42);
        }
        assert_eq!(entries[0].text, "line a");
        assert_eq!(entries[1].text, "line b");
        assert_eq!(entries[2].text, "line c");
    }

    #[test]
    fn format_log_entry_general_no_tick() {
        let e = make_entry(LogCategory::General, 0, "hello");
        assert_eq!(format_log_entry(&e), "[GENERAL] hello");
    }

    #[test]
    fn format_log_entry_with_nonzero_tick_prefixes_tick() {
        let e = make_entry(LogCategory::Combat, 1234, "hero swings");
        assert_eq!(format_log_entry(&e), "[1234] [COMBAT] hero swings");
    }

    #[test]
    fn filter_keeps_only_active_categories() {
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "hit"),
            make_entry(LogCategory::Movement, 0, "step"),
            make_entry(LogCategory::Quest, 0, "flag"),
            make_entry(LogCategory::Ai, 0, "think"),
        ];
        let active: std::collections::HashSet<LogCategory> =
            [LogCategory::Combat, LogCategory::Quest].iter().copied().collect();
        let kept = filter_log_entries(&entries, &active);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].text, "hit");
        assert_eq!(kept[1].text, "flag");
    }

    #[test]
    fn filter_empty_active_set_hides_everything() {
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "a"),
            make_entry(LogCategory::General, 0, "b"),
        ];
        let active: std::collections::HashSet<LogCategory> = Default::default();
        let kept = filter_log_entries(&entries, &active);
        assert!(kept.is_empty());
    }

    #[test]
    fn filter_default_active_set_hides_noisy_categories() {
        let active: std::collections::HashSet<LogCategory> = LogCategory::ALL
            .iter()
            .copied()
            .filter(|c| c.default_enabled())
            .collect();
        let entries = vec![
            make_entry(LogCategory::Combat, 0, "shown"),
            make_entry(LogCategory::Movement, 0, "hidden"),
            make_entry(LogCategory::Ai, 0, "hidden"),
            make_entry(LogCategory::Rendering, 0, "hidden"),
            make_entry(LogCategory::Animation, 0, "hidden"),
            make_entry(LogCategory::Time, 0, "hidden"),
            make_entry(LogCategory::General, 0, "shown"),
        ];
        let kept = filter_log_entries(&entries, &active);
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|e| e.text == "shown"));
    }

    #[test]
    fn parse_category_case_insensitive() {
        assert_eq!(parse_category("combat"), Some(LogCategory::Combat));
        assert_eq!(parse_category("COMBAT"), Some(LogCategory::Combat));
        assert_eq!(parse_category("Movement"), Some(LogCategory::Movement));
        assert_eq!(parse_category("ai"), Some(LogCategory::Ai));
        assert_eq!(parse_category("nonsense"), None);
        assert_eq!(parse_category(""), None);
    }

    #[test]
    fn filter_modal_lines_marks_cursor_and_state() {
        let mut active = std::collections::HashSet::new();
        active.insert(LogCategory::Combat);
        active.insert(LogCategory::Quest);
        let lines = filter_modal_lines(0, &active);
        assert_eq!(lines.len(), LogCategory::ALL.len());
        assert!(lines[0].starts_with("> [ON] "));
        assert!(lines[0].ends_with("COMBAT"));
        assert!(lines[1].starts_with("  [OFF] "));
        assert!(lines[1].ends_with("ENCOUNTER"));
        assert!(lines[2].starts_with("  [ON] "));
        assert!(lines[2].ends_with("QUEST"));
    }

    #[test]
    fn filter_modal_lines_cursor_moves() {
        let active = std::collections::HashSet::new();
        let lines = filter_modal_lines(5, &active);
        assert!(lines[5].starts_with("> "));
        for (i, line) in lines.iter().enumerate() {
            if i != 5 {
                assert!(line.starts_with("  "), "row {i} should not have cursor: {line:?}");
            }
        }
    }

    // Silence unused-warning for the helper used only by cmd_god / render.
    #[test]
    fn build_god_str_empty_for_zero_flags() {
        assert_eq!(build_god_str(0), "");
    }
}
