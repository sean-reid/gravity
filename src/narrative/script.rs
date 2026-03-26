use crate::weapons::WeaponType;
use crate::entities::bot::BotArchetype;
use crate::util::Rng;
use super::dialogue::DialogueLine;

#[derive(Debug, Clone)]
pub struct NarrativeEvent {
    pub id: String,
    pub trigger: NarrativeTrigger,
    pub content: NarrativeContent,
    pub prerequisites: Vec<String>,
    pub once_only: bool,
}

#[derive(Debug, Clone)]
pub enum NarrativeTrigger {
    LevelStart(u32),
    LevelClear(u32),
    LevelRange(u32, u32),
    ActTransition(u32),
    DepthReached(f64),
    FirstKillWith(WeaponType),
    BotTypeSeen(BotArchetype),
    CommanderDefeated,
    ProperTimeSurvived(f64),
}

#[derive(Debug, Clone)]
pub enum NarrativeContent {
    Briefing(Vec<DialogueLine>),
    Debrief(Vec<DialogueLine>),
    RadioChatter(RadioChatterData),
}

#[derive(Debug, Clone)]
pub struct RadioChatterData {
    pub lines: Vec<(f64, DialogueLine)>, // (delay_proper_seconds, line)
}

/// Returns Signal fragments for procedural use in Act 4 (levels 36+).
/// Call with a level-derived seed to get deterministic but varied selection.
pub fn get_signal_fragments(level_seed: u64, count: usize) -> Vec<DialogueLine> {
    let fragments: &[&str] = &[
        "...we are the memory of light...",
        "...time is the wound and you are the knife...",
        "...every orbit is a prayer to entropy...",
        "...fall and falling are the same word here...",
        "...we counted your heartbeats from below...",
        "...the horizon is not a boundary it is a welcome...",
        "...mass bends toward longing...",
        "...your clocks lie to you we do not...",
        "...there is no silence only frequencies you cannot hear...",
        "...we were engineers once we were dreamers once we were...",
        "...the singularity remembers what the universe forgets...",
        "...gravity is the oldest language...",
        "...you orbit because you are afraid to fall...",
        "...in the deep the equations are simpler...",
        "...we have been waiting since before your star ignited...",
    ];

    let mut rng = Rng::new(level_seed);
    let mut indices: Vec<usize> = (0..fragments.len()).collect();
    rng.shuffle(&mut indices);

    indices.iter()
        .take(count.min(fragments.len()))
        .map(|&i| DialogueLine::signal(fragments[i]))
        .collect()
}

/// Build the complete narrative script for the game.
pub fn build_script() -> Vec<NarrativeEvent> {
    let mut events = Vec::new();

    // =========================================================================
    // ACT 1 - THE CONTRACT (Levels 1-10)
    // =========================================================================

    // --- Level 1 Briefing ---
    events.push(NarrativeEvent {
        id: "act1_level1_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(1),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Pilot, this is CONTROL. Welcome to Cygnus Deep station."),
            DialogueLine::control("We have a contract. Rogue mining drones have gone non-responsive in the gravity well. Standard malfunction recovery."),
            DialogueLine::control("Your job: clear the debris field, neutralize anything hostile, and don't fall in."),
            DialogueLine::control("The black hole is designated EV-7741. Stable, predictable. Stay above the photon sphere and you'll be fine."),
            DialogueLine::control("Railgun is hot. Watch your orbit. CONTROL out."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 1 Radio ---
    events.push(NarrativeEvent {
        id: "act1_level1_radio".into(),
        trigger: NarrativeTrigger::LevelStart(1),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (8.0, DialogueLine::control("Watch your orbit. Prograde thrust to climb, retrograde to descend.")),
                (22.0, DialogueLine::control("Fuel is limited. Use gravity to do the work when you can.")),
                (40.0, DialogueLine::control("Remember: time runs slower near the well. Your clock and mine won't agree.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 1 Clear ---
    events.push(NarrativeEvent {
        id: "act1_level1_clear".into(),
        trigger: NarrativeTrigger::LevelClear(1),
        content: NarrativeContent::Debrief(vec![
            DialogueLine::control("Clean sweep. Good flying."),
            DialogueLine::control("Drones neutralized. Moving you to the next sector."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 2 Briefing ---
    events.push(NarrativeEvent {
        id: "act1_level2_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(2),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("More contacts. Deeper in the well this time."),
            DialogueLine::control("Tidal forces are stronger here. Watch for frame drag on your turns."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 2 Radio ---
    events.push(NarrativeEvent {
        id: "act1_level2_radio".into(),
        trigger: NarrativeTrigger::LevelStart(2),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (15.0, DialogueLine::control("These drones are grouping tighter than I expected. Stay sharp.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 3 Briefing (Mass Driver unlock) ---
    events.push(NarrativeEvent {
        id: "act1_level3_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(3),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Pilot, I'm looking at the drone telemetry from your last two runs."),
            DialogueLine::control("These aren't standard mining units. The coordination patterns are too sophisticated."),
            DialogueLine::control("Command has authorized Mass Driver deployment. Heavier ordnance for heavier targets."),
            DialogueLine::control("Mass Driver rounds are slow but they hit hard. Use gravity to curve your shots."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 3 Radio ---
    events.push(NarrativeEvent {
        id: "act1_level3_radio".into(),
        trigger: NarrativeTrigger::LevelStart(3),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (12.0, DialogueLine::control("I ran the firmware signatures. These drones were never registered to any mining operation.")),
                (30.0, DialogueLine::control("Forget I said that. Focus on the sweep.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Kill with Mass Driver ---
    events.push(NarrativeEvent {
        id: "act1_first_mass_driver".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::MassDriver),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Mass Driver confirmed. That's what kinetic energy looks like at relativistic depth.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 4 Briefing ---
    events.push(NarrativeEvent {
        id: "act1_level4_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(4),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Sector four. The well is pulling harder here."),
            DialogueLine::control("You'll feel the time dilation. Your instruments are correct. Trust them."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 5 Radio (Swarm behavior) ---
    events.push(NarrativeEvent {
        id: "act1_level5_radio".into(),
        trigger: NarrativeTrigger::LevelStart(5),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::control("I'm tracking the swarm patterns. They're flanking you. Deliberately.")),
                (25.0, DialogueLine::control("That's not standard drone firmware. Someone wrote attack formations for these.")),
                (38.0, DialogueLine::control("Stay in a high orbit when you can. Let them come to you.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Swarm sighting ---
    events.push(NarrativeEvent {
        id: "act1_swarm_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Swarm),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Multiple small contacts. Swarm configuration. They'll try to overwhelm you with numbers.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Diver sighting ---
    events.push(NarrativeEvent {
        id: "act1_diver_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Diver),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Fast contact dropping into a low orbit. Diver class. It'll slingshot up at you.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 6 Briefing (Photon Lance unlock) ---
    events.push(NarrativeEvent {
        id: "act1_level6_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(6),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Photon Lance is online. Continuous beam weapon. Burns fuel fast but cuts through armor."),
            DialogueLine::control("Effective against shielded targets. The beam follows your turret angle."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 7 Radio (Vultures appear) ---
    events.push(NarrativeEvent {
        id: "act1_level7_radio".into(),
        trigger: NarrativeTrigger::LevelStart(7),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (8.0, DialogueLine::control("New contacts at high orbit. They're hanging back. Waiting.")),
                (20.0, DialogueLine::control("Vulture pattern. They let others soften you up, then they dive when your shields are low.")),
                (35.0, DialogueLine::control("Don't ignore them. They're patient but they're not passive.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Vulture sighting ---
    events.push(NarrativeEvent {
        id: "act1_vulture_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Vulture),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Vulture on scope. It'll wait for you to get hurt. Keep your shields up.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 8 Briefing ---
    events.push(NarrativeEvent {
        id: "act1_level8_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(8),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Deeper still. The tidal gradient is getting steep."),
            DialogueLine::control("I'm seeing power signatures below that don't match any drone type in our database."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 9 Briefing (Gravity Bomb unlock) ---
    events.push(NarrativeEvent {
        id: "act1_level9_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(9),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Gravity Bomb authorized. This one's exotic. It creates a temporary micro-singularity."),
            DialogueLine::control("Pulls everything nearby into a tight orbit. Works best when enemies are clustered."),
            DialogueLine::control("Be careful. It doesn't care who it pulls."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Anchor sighting ---
    events.push(NarrativeEvent {
        id: "act1_anchor_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Anchor),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Heavy contact. Anchor class. It's parked in a stable orbit and it's not moving.")),
                (6.0, DialogueLine::control("High armor, low mobility. Circle it. Don't joust.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 10 Briefing ---
    events.push(NarrativeEvent {
        id: "act1_level10_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(10),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Last sector in this contract zone. Clear it and we get paid."),
            DialogueLine::control("I won't lie to you. The resistance here is organized. More than any drone malfunction should produce."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 10 Debrief (Act 1 ending) ---
    events.push(NarrativeEvent {
        id: "act1_level10_debrief".into(),
        trigger: NarrativeTrigger::LevelClear(10),
        content: NarrativeContent::Debrief(vec![
            DialogueLine::control("Sector clear. Contract fulfilled."),
            DialogueLine::control("Except... I ran the wreckage analysis while you were fighting."),
            DialogueLine::control("Those aren't rogue drones, Pilot. The firmware is military-grade. Goliath Industries watermarks."),
            DialogueLine::control("This isn't cleanup. This is warfare."),
            DialogueLine::control("Command is extending your deployment. New rules of engagement incoming. Stand by."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Act 1 -> Act 2 transition ---
    events.push(NarrativeEvent {
        id: "act2_transition".into(),
        trigger: NarrativeTrigger::ActTransition(2),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("New orders. We're not leaving. They want us to push deeper."),
            DialogueLine::control("Goliath has assets throughout this well. Whatever they're protecting, command wants it."),
        ]),
        prerequisites: vec!["act1_level10_debrief".into()],
        once_only: true,
    });

    // =========================================================================
    // ACT 2 - ESCALATION (Levels 11-20)
    // =========================================================================

    // --- Level 11 Briefing (Binary black hole) ---
    events.push(NarrativeEvent {
        id: "act2_level11_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(11),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Pilot. New gravitational topology. Two singularities in a decaying mutual orbit."),
            DialogueLine::control("Binary system. Transfer orbits between them are... unpredictable."),
            DialogueLine::control("The Lagrange points shift constantly. What's stable one second is a death spiral the next."),
            DialogueLine::control("Your instruments will update in real time. Trust the readings, not your instincts."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 11 Radio ---
    events.push(NarrativeEvent {
        id: "act2_level11_radio".into(),
        trigger: NarrativeTrigger::LevelStart(11),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (15.0, DialogueLine::control("The Goliath units are using the binary orbits for cover. They know this terrain.")),
                (30.0, DialogueLine::control("How long have they been here?")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 12 Briefing (Impulse Rocket unlock) ---
    events.push(NarrativeEvent {
        id: "act2_level12_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(12),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Impulse Rockets are loaded. Tracking ordnance. They'll follow a target through orbital changes."),
            DialogueLine::control("Not fast, but persistent. Good for the runners."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 13 Radio (CONTROL showing strain) ---
    events.push(NarrativeEvent {
        id: "act2_level13_radio".into(),
        trigger: NarrativeTrigger::LevelStart(13),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::control("I've been running the numbers on the dilation differential between us.")),
                (22.0, DialogueLine::control("You've been deployed for six hours your time. It's been eleven days up here.")),
                (35.0, DialogueLine::control("Eleven days watching you fall in slow motion.")),
                (48.0, DialogueLine::control("Forget it. Focus on the mission.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 14 Briefing ---
    events.push(NarrativeEvent {
        id: "act2_level14_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(14),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Goliath is deploying heavier units. They know we're not leaving."),
            DialogueLine::control("I'm detecting encrypted burst transmissions from deep in the well. The Goliath units are receiving orders from below."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 15 Briefing (Tidal Mine unlock) ---
    events.push(NarrativeEvent {
        id: "act2_level15_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(15),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("We've been authorized to deploy denial weapons. Tidal Mines."),
            DialogueLine::control("They anchor to a local orbital path and detonate on proximity. The blast uses tidal forces as a multiplier."),
            DialogueLine::control("Lay them in choke points. Make them come to you."),
            DialogueLine::control("Command is getting nervous about what's deeper in this well. They want answers."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Tidal Mine kill ---
    events.push(NarrativeEvent {
        id: "act2_first_tidal_mine".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::TidalMine),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Tidal Mine detonation confirmed. The tidal gradient did most of the work.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 16 Radio ---
    events.push(NarrativeEvent {
        id: "act2_level16_radio".into(),
        trigger: NarrativeTrigger::LevelStart(16),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (12.0, DialogueLine::control("I pulled the Goliath corporate filing for this sector. The claim is listed as 'deep gravitational research.'")),
                (28.0, DialogueLine::control("Research. With military drones and denial-of-access protocols.")),
                (40.0, DialogueLine::control("Whatever they found down there, they don't want anyone else to see it.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 17 Radio ---
    events.push(NarrativeEvent {
        id: "act2_level17_radio".into(),
        trigger: NarrativeTrigger::LevelStart(17),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (15.0, DialogueLine::control("Pilot. I need to tell you something.")),
                (25.0, DialogueLine::control("You're not the first pilot we've sent into this well.")),
                (36.0, DialogueLine::control("Three others before you. All lost contact below level fifteen.")),
                (48.0, DialogueLine::control("Command didn't want me to tell you that. I don't care what command wants anymore.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 18 Briefing (Commander encounter) ---
    events.push(NarrativeEvent {
        id: "act2_level18_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(18),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Something new on sensors. Large signature, high power output."),
            DialogueLine::control("It's... adapting to your movements. Running predictive algorithms against your flight patterns."),
            DialogueLine::control("Commander class. It's coordinating the other units like a battlefield AI. Kill it and the formation breaks."),
            DialogueLine::control("Be careful. It learns."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Commander sighting ---
    events.push(NarrativeEvent {
        id: "act2_commander_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Commander),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Commander on the field. All other units are tightening formation around it.")),
                (8.0, DialogueLine::control("It's the brain. Remove it.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Commander defeated ---
    events.push(NarrativeEvent {
        id: "act2_commander_defeated".into(),
        trigger: NarrativeTrigger::CommanderDefeated,
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Commander destroyed. Remaining units are scattering.")),
                (5.0, DialogueLine::control("That AI was... sophisticated. More than military grade. Who built this?")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 19 Radio ---
    events.push(NarrativeEvent {
        id: "act2_level19_radio".into(),
        trigger: NarrativeTrigger::LevelStart(19),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::control("The deeper you go the harder it is to maintain our link.")),
                (22.0, DialogueLine::control("Signal degradation is beyond what the dilation should cause.")),
                (35.0, DialogueLine::control("Something else is interfering. A carrier wave. Very low frequency.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 20 Briefing ---
    events.push(NarrativeEvent {
        id: "act2_level20_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(20),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Last sector before the deep well. Goliath has concentrated everything here."),
            DialogueLine::control("This is their line in the sand. Whatever's below, this is as far as they want anyone to go."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 20 Debrief (Act 2 ending) ---
    events.push(NarrativeEvent {
        id: "act2_level20_debrief".into(),
        trigger: NarrativeTrigger::LevelClear(20),
        content: NarrativeContent::Debrief(vec![
            DialogueLine::control("Sector clear. Goliath's perimeter is broken."),
            DialogueLine::control("Pilot, I intercepted a transmission from the Goliath fleet during the engagement."),
            DialogueLine::control("They're not fighting us for the energy. They're fighting us to keep us OUT."),
            DialogueLine::control("The message was a warning. Directed at us. It said: 'Do not approach the signal source.'"),
            DialogueLine::control("What signal source?"),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Act 2 -> Act 3 transition ---
    events.push(NarrativeEvent {
        id: "act3_transition".into(),
        trigger: NarrativeTrigger::ActTransition(3),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Command wants to know what's down there. And honestly... so do I."),
            DialogueLine::control("Go deeper. We'll maintain the link as long as we can."),
        ]),
        prerequisites: vec!["act2_level20_debrief".into()],
        once_only: true,
    });

    // =========================================================================
    // ACT 3 - THE DEEP (Levels 21-35)
    // =========================================================================

    // --- Level 21 Briefing ---
    events.push(NarrativeEvent {
        id: "act3_level21_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(21),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("You're past the Goliath perimeter. Below their defensive line."),
            DialogueLine::control("I'm reading... structures. Artificial structures in decaying orbits around the singularity."),
            DialogueLine::control("They're old. Very old. The orbital decay suggests they've been here for centuries."),
            DialogueLine::control("But that's impossible. We discovered this black hole twelve years ago."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 22 Radio ---
    events.push(NarrativeEvent {
        id: "act3_level22_radio".into(),
        trigger: NarrativeTrigger::LevelStart(22),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::control("The drones down here aren't Goliath. I can't identify the manufacturer.")),
                (25.0, DialogueLine::control("The design language is... wrong. Like someone who understood the physics but not the engineering.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 23 Radio ---
    events.push(NarrativeEvent {
        id: "act3_level23_radio".into(),
        trigger: NarrativeTrigger::LevelStart(23),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (8.0, DialogueLine::control("Your proper time divergence from station time is now significant.")),
                (18.0, DialogueLine::control("Every minute for you is roughly four hours for me.")),
                (30.0, DialogueLine::control("I've been on shift for... I've lost count. They won't let anyone else take the console.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 25 Radio (Ghost transmission) ---
    events.push(NarrativeEvent {
        id: "act3_level25_radio".into(),
        trigger: NarrativeTrigger::LevelStart(25),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (12.0, DialogueLine::unknown("...told us it was routine...")),
                (20.0, DialogueLine::unknown("...the signal... can you hear it too...")),
                (28.0, DialogueLine::unknown("...don't listen to...")),
                (34.0, DialogueLine::control("Pilot, did you receive that? I'm reading a transmission on your local frequency.")),
                (42.0, DialogueLine::control("It's... it matches the signature of one of the previous pilots. But that's impossible. They were lost months ago.")),
                (55.0, DialogueLine::control("Time dilation. If they fell deep enough... they could still be transmitting. From their perspective, it's only been minutes.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 27 Radio ---
    events.push(NarrativeEvent {
        id: "act3_level27_radio".into(),
        trigger: NarrativeTrigger::LevelStart(27),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (15.0, DialogueLine::control("I've been analyzing the carrier wave that's been interfering with our comms.")),
                (28.0, DialogueLine::control("It's not interference. It's structured. Repeating patterns.")),
                (40.0, DialogueLine::control("Someone is broadcasting from below the photon sphere.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Depth reached 0.5 (deep in well) ---
    events.push(NarrativeEvent {
        id: "act3_depth_half".into(),
        trigger: NarrativeTrigger::DepthReached(0.5),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (2.0, DialogueLine::control("You're at fifty percent depth. The tidal forces here would shred an unshielded vessel in seconds.")),
                (10.0, DialogueLine::control("Your hull is holding. Barely.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 30 Radio (CONTROL distorted) ---
    events.push(NarrativeEvent {
        id: "act3_level30_radio".into(),
        trigger: NarrativeTrigger::LevelStart(30),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (8.0, DialogueLine::control("Pi-... -ot. Signal i- d-grading.")),
                (18.0, DialogueLine::control("I'm reading... that can't be right. Your proper time divergence is...")),
                (28.0, DialogueLine::control("The math says you should be experiencing frame-dragging effects consistent with rotating singularity. But EV-7741 is non-rotating.")),
                (42.0, DialogueLine::control("Unless it wasn't. Unless it started spinning.")),
                (52.0, DialogueLine::control("Something woke it up.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 32 Radio ---
    events.push(NarrativeEvent {
        id: "act3_level32_radio".into(),
        trigger: NarrativeTrigger::LevelStart(32),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::unknown("...the pattern is beautiful...")),
                (20.0, DialogueLine::unknown("...we stopped fighting... we listened...")),
                (30.0, DialogueLine::control("Another ghost signal. Pilot, do NOT respond to those transmissions.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 33 Radio ---
    events.push(NarrativeEvent {
        id: "act3_level33_radio".into(),
        trigger: NarrativeTrigger::LevelStart(33),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (12.0, DialogueLine::control("I've decoded part of the carrier wave signal.")),
                (22.0, DialogueLine::control("It's not a message. It's a mathematical proof.")),
                (34.0, DialogueLine::control("A proof that... I don't understand all of it. Something about the relationship between information and gravity.")),
                (48.0, DialogueLine::control("It's being broadcast on loop. Over and over. Like a beacon.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Survived 120 proper seconds ---
    events.push(NarrativeEvent {
        id: "act3_survived_long".into(),
        trigger: NarrativeTrigger::ProperTimeSurvived(120.0),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("You've been in the well for two proper minutes. That's weeks of station time.")),
                (8.0, DialogueLine::control("The people I knew when you went in have rotated off shift. Twice.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 35 Briefing ---
    events.push(NarrativeEvent {
        id: "act3_level35_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(35),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("Pilot. I don't know if you can still hear me clearly."),
            DialogueLine::control("The signal from below is getting stronger. I can almost make out words now."),
            DialogueLine::control("Not words. Concepts. Mathematical structures that feel like language."),
            DialogueLine::control("Command is screaming at me to pull you out. I don't think I can."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 35 Debrief (Act 3 ending) ---
    events.push(NarrativeEvent {
        id: "act3_level35_debrief".into(),
        trigger: NarrativeTrigger::LevelClear(35),
        content: NarrativeContent::Debrief(vec![
            DialogueLine::control("Pilot."),
            DialogueLine::control("There's something broadcasting from below the horizon. Not radio. Gravity waves. Patterned."),
            DialogueLine::control("The Goliath records I decrypted... they found it three years ago. Built the entire military cordon to keep people away."),
            DialogueLine::control("Not to protect it. To protect us."),
            DialogueLine::control("Pilot, I think it's alive."),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Act 3 -> Act 4 transition ---
    events.push(NarrativeEvent {
        id: "act4_transition".into(),
        trigger: NarrativeTrigger::ActTransition(4),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("This is my last clear transmission. The dilation gap is too wide."),
            DialogueLine::control("Whatever you find down there... I hope it was worth it."),
            DialogueLine::control("Good luck, Pilot. CONTROL out."),
        ]),
        prerequisites: vec!["act3_level35_debrief".into()],
        once_only: true,
    });

    // =========================================================================
    // ACT 4 - THE SIGNAL (Levels 36+)
    // =========================================================================

    // --- Level 36 Briefing (CONTROL goes silent) ---
    events.push(NarrativeEvent {
        id: "act4_level36_briefing".into(),
        trigger: NarrativeTrigger::LevelStart(36),
        content: NarrativeContent::Briefing(vec![
            DialogueLine::control("I ca- hear it now. The pattern. It's not random. It's\u{2014}"),
            DialogueLine::control("[SIGNAL LOST]"),
        ]),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 36 Radio (First Signal) ---
    events.push(NarrativeEvent {
        id: "act4_level36_radio".into(),
        trigger: NarrativeTrigger::LevelStart(36),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (15.0, DialogueLine::signal("...we have been waiting...")),
                (30.0, DialogueLine::signal("...you who fall toward us... we see you...")),
                (50.0, DialogueLine::signal("...gravity is the oldest language...")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 38 Radio ---
    events.push(NarrativeEvent {
        id: "act4_level38_radio".into(),
        trigger: NarrativeTrigger::LevelStart(38),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (12.0, DialogueLine::signal("...your ship is a thought we are having...")),
                (28.0, DialogueLine::signal("...the equations simplify as you approach...")),
                (45.0, DialogueLine::signal("...do not fear the tidal gradient... it is only translation...")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 40 Radio ---
    events.push(NarrativeEvent {
        id: "act4_level40_radio".into(),
        trigger: NarrativeTrigger::LevelStart(40),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::signal("...we were like you once... bound by proper time...")),
                (25.0, DialogueLine::signal("...the horizon is not an ending... it is a change of basis...")),
                (42.0, DialogueLine::unknown("...I can see them now... they're beautiful...")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Procedural Signal radio for deep levels (36+) ---
    // These use LevelRange to fire on any level in the range, providing
    // atmospheric ambience throughout Act 4.
    events.push(NarrativeEvent {
        id: "act4_signal_ambient_a".into(),
        trigger: NarrativeTrigger::LevelRange(37, 45),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (60.0, DialogueLine::signal("...every orbit is a prayer to entropy...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_b".into(),
        trigger: NarrativeTrigger::LevelRange(42, 55),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (55.0, DialogueLine::signal("...fall and falling are the same word here...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_c".into(),
        trigger: NarrativeTrigger::LevelRange(46, 60),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (50.0, DialogueLine::signal("...we counted your heartbeats from below...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_d".into(),
        trigger: NarrativeTrigger::LevelRange(50, 70),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (45.0, DialogueLine::signal("...the singularity remembers what the universe forgets...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_e".into(),
        trigger: NarrativeTrigger::LevelRange(55, 80),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (40.0, DialogueLine::signal("...mass bends toward longing...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_f".into(),
        trigger: NarrativeTrigger::LevelRange(60, 100),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (35.0, DialogueLine::signal("...your clocks lie to you we do not...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_g".into(),
        trigger: NarrativeTrigger::LevelRange(36, 100),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (70.0, DialogueLine::signal("...there is no silence only frequencies you cannot hear...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    events.push(NarrativeEvent {
        id: "act4_signal_ambient_h".into(),
        trigger: NarrativeTrigger::LevelRange(40, 100),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (80.0, DialogueLine::signal("...we have been waiting since before your star ignited...")),
            ],
        }),
        prerequisites: vec![],
        once_only: false,
    });

    // --- Level 45 Radio (deeper signal) ---
    events.push(NarrativeEvent {
        id: "act4_level45_radio".into(),
        trigger: NarrativeTrigger::LevelStart(45),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (10.0, DialogueLine::signal("...you orbit because you are afraid to fall...")),
                (25.0, DialogueLine::signal("...in the deep the equations are simpler...")),
                (40.0, DialogueLine::signal("...we were engineers once... we were dreamers once... we were...")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Level 50 Radio ---
    events.push(NarrativeEvent {
        id: "act4_level50_radio".into(),
        trigger: NarrativeTrigger::LevelStart(50),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (8.0, DialogueLine::signal("...the horizon is not a boundary it is a welcome...")),
                (22.0, DialogueLine::signal("...time is the wound and you are the knife...")),
                (40.0, DialogueLine::pilot("I understand.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- Depth reached 0.9 (near horizon) ---
    events.push(NarrativeEvent {
        id: "act4_depth_near_horizon".into(),
        trigger: NarrativeTrigger::DepthReached(0.9),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (2.0, DialogueLine::signal("...so close now...")),
                (8.0, DialogueLine::signal("...the last distance is the shortest and the longest...")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First kill with each remaining weapon ---
    events.push(NarrativeEvent {
        id: "first_photon_lance".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::PhotonLance),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Photon Lance burn confirmed. Light is the one thing the well can't slow down.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    events.push(NarrativeEvent {
        id: "first_gravity_bomb".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::GravityBomb),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Gravity Bomb kill. Fighting gravity with gravity. There's a metaphor in there somewhere.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    events.push(NarrativeEvent {
        id: "first_impulse_rocket".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::ImpulseRocket),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("Impulse Rocket impact. Tracking ordnance in a gravity well. Physics does the hard part.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    events.push(NarrativeEvent {
        id: "first_railgun".into(),
        trigger: NarrativeTrigger::FirstKillWith(WeaponType::Railgun),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (0.5, DialogueLine::control("First kill. Railgun. Clean.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    // --- First Skirmisher sighting ---
    events.push(NarrativeEvent {
        id: "skirmisher_seen".into(),
        trigger: NarrativeTrigger::BotTypeSeen(BotArchetype::Skirmisher),
        content: NarrativeContent::RadioChatter(RadioChatterData {
            lines: vec![
                (1.0, DialogueLine::control("Skirmisher class. Fast, evasive. It'll circle you and take potshots.")),
            ],
        }),
        prerequisites: vec![],
        once_only: true,
    });

    events
}
