use crate::constants::DEFAULT_MAX_NUMBER_PLAYERS;
use crate::definitions::tag::model_collision_geometry::ModelCollisionGeometryBSP;
use crate::definitions::tag::object::{Object, ObjectType};
use crate::definitions::tag::scenario::{Scenario, ScenarioBSPSwitchTriggerVolume, ScenarioSpawnType, ScenarioType};
use crate::definitions::tag::scenario_structure_bsp::ScenarioStructureBSP;
use crate::postprocess::Action;
use crate::{EditableCompositeTagField, EditableIndexedTagField, EditableTagField, PostprocessError, PostprocessState, PostprocessWarningType, ReflexiveIndex, TagPath};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use funnel_web::collision_bsp::CollisionBSPFunctions;
use funnel_web::constants::TICK_RATE;
use funnel_web::id::Index;
use funnel_web::nudge::fix_decimal_rounding;
use funnel_web::vector::{Euler3D, Matrix4x3, Vector3D};

pub(crate) fn postprocess_scenario(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    // Do not postprocess anything that's not the main scenario tag
    if action.postprocess() && scenario as *const Scenario != state.scenario_tag() as *const Scenario {
        return Ok(())
    }

    check_duplicate_object_names(scenario)?;
    merge_scenarios(scenario, action, tag_path, state)?;

    let all_bsps = get_all_bsps_for_postprocessing(scenario, action, state)?;

    check_palettes(scenario, action, tag_path, state)?;
    compile_scripts(scenario, action, tag_path, state)?;
    check_player_spawns(scenario, action, tag_path, state);
    set_bsp_indices_for_scenery(scenario, action, tag_path, state, &all_bsps);
    fixup_object_names(scenario, action)?;
    set_surface_indices_for_ai(scenario, action, tag_path, state, &all_bsps)?;
    generate_bsp_trigger_volumes(scenario, action, tag_path, state, &all_bsps)?;
    postprocess_cutscene_titles(scenario, action);

    Ok(())
}

fn postprocess_cutscene_titles(scenario: &mut Scenario, action: Action) {
    if action.postprocess() {
        for title in &mut scenario.cutscene_titles {
            title.fade_in_time *= TICK_RATE;
            title.fade_out_time *= TICK_RATE;
            title.up_time *= TICK_RATE;
            title.up_time += title.fade_in_time;
        }
    }
    else if action.unpostprocess() {
        for title in &mut scenario.cutscene_titles {
            // has to be done in reverse
            title.up_time -= title.fade_in_time;
            title.up_time /= TICK_RATE;
            title.fade_out_time /= TICK_RATE;
            title.fade_in_time /= TICK_RATE;
        }
    }

    if action.nudge() {
        for title in &mut scenario.cutscene_titles {
            title.up_time = fix_decimal_rounding(title.up_time);
            title.fade_out_time = fix_decimal_rounding(title.fade_out_time);
            title.fade_in_time = fix_decimal_rounding(title.fade_in_time);
        }
    }
}

macro_rules! get_objects_and_palettes {
    ($scenario:expr) => {
        [
            (&$scenario.bipeds as &dyn EditableIndexedTagField, &$scenario.biped_palette as &dyn EditableIndexedTagField, ObjectType::Biped),
            (&$scenario.vehicles as &dyn EditableIndexedTagField, &$scenario.vehicle_palette as &dyn EditableIndexedTagField, ObjectType::Vehicle),
            (&$scenario.weapons as &dyn EditableIndexedTagField, &$scenario.weapon_palette as &dyn EditableIndexedTagField, ObjectType::Weapon),
            (&$scenario.equipment as &dyn EditableIndexedTagField, &$scenario.equipment_palette as &dyn EditableIndexedTagField, ObjectType::Equipment),
            (&$scenario.controls as &dyn EditableIndexedTagField, &$scenario.control_palette as &dyn EditableIndexedTagField, ObjectType::DeviceControl),
            (&$scenario.light_fixtures as &dyn EditableIndexedTagField, &$scenario.light_fixture_palette as &dyn EditableIndexedTagField, ObjectType::DeviceLightFixture),
            (&$scenario.machines as &dyn EditableIndexedTagField, &$scenario.machine_palette as &dyn EditableIndexedTagField, ObjectType::DeviceMachine),
            (&$scenario.scenery as &dyn EditableIndexedTagField, &$scenario.scenery_palette as &dyn EditableIndexedTagField, ObjectType::Scenery),
            (&$scenario.sound_scenery as &dyn EditableIndexedTagField, &$scenario.sound_scenery_palette as &dyn EditableIndexedTagField, ObjectType::SoundScenery),
        ]
    };
}

fn set_surface_indices_for_ai(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState, all_bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    todo!()
}

fn generate_bsp_trigger_volumes(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState, all_bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for (index, trigger_volume) in scenario.trigger_volumes.iter().enumerate() {
        let trigger_volume_name = trigger_volume.name.as_str();
        let prefix = "bsp";
        if !trigger_volume_name.starts_with(prefix) {
            continue
        }

        let source_destination = &trigger_volume_name[prefix.len()..];
        let mut split = source_destination.split(",");
        let warn_prefix = format_args!("Trigger volume {trigger_volume_name} starts with \"bsp\" but is not correctly formatted as a BSP trigger volume");

        let Some(source) = split.next() else {
            state.warn(tag_path, format_args!("{warn_prefix} (missing source)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        let Some(destination) = split.next() else {
            state.warn(tag_path, format_args!("{warn_prefix} (missing destination)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        let None = split.next() else {
            state.warn(tag_path, format_args!("{warn_prefix} (extraneous commas found)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        let Ok(source) = source.parse::<usize>() else {
            state.warn(tag_path, format_args!("{warn_prefix} (source is not a non-negative integer)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        let Ok(destination) = destination.parse::<usize>() else {
            state.warn(tag_path, format_args!("{warn_prefix} (destination is not a non-negative integer)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        if source >= all_bsps.len() {
            state.warn(tag_path, format_args!("{warn_prefix} (source exceeds BSP count)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        if destination >= all_bsps.len() {
            state.warn(tag_path, format_args!("{warn_prefix} (destination exceeds BSP count)."), PostprocessWarningType::InvalidBSPTriggerVolumeName);
            continue
        };

        let mut safe_flag = ReflexiveIndex::new();
        for (flag_index, flag) in scenario.cutscene_flags.iter().enumerate() {
            if !flag.name.as_str().eq_ignore_ascii_case("safe") {
                continue
            }

            if trigger_volume.test_point(flag.position) {
                safe_flag = match Index::from_usize(flag_index) {
                    Some(n) => ReflexiveIndex(n),
                    None => fail_postprocess!("Maximum number of referenceable cutscene flags exceeded.")
                };
                break
            }
        }

        scenario.bsp_switch_trigger_volumes.push(ScenarioBSPSwitchTriggerVolume {
            trigger_volume: match Index::from_usize(index) {
                Some(n) => n.into(),
                None => fail_postprocess!("Maximum number of referenceable trigger volumes exceeded.")
            },
            source: Index::from_usize(source).expect("BSP count should be checked").into(),
            destination: Index::from_usize(destination).expect("BSP count should be checked").into(),
            safe_flag,
        });
    }

    Ok(())
}

fn fixup_object_names(scenario: &mut Scenario, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let everything = get_objects_and_palettes!(scenario);

    'outer: for (name_index, object_name) in scenario.object_names.iter_mut().enumerate() {
        for (spawns, _, object_type) in everything {
            for (spawn_index, spawn) in spawns.iter().enumerate() {
                let s = spawn.get_composite().expect("spawns must be composite");
                let Some(name) = s.get_field("name")
                    .expect("no name???")
                    .get_reflexive_index()
                    .expect("no index???")
                    .get_index()
                    .index() else {
                    continue
                };

                if name == name_index {
                    object_name.object_type = object_type;

                    match Index::from_usize(spawn_index) {
                        Some(n) => object_name.object_index = n,
                        None => fail_postprocess!("Object name #{name_index} ({}) used by an {object_type} index >65535", object_name.name)
                    }

                    continue 'outer
                }
            }
        }
    }

    Ok(())
}

fn merge_scenarios(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for child_scenario in core::mem::take(&mut scenario.child_scenarios) {
        let Some(child_scenario_path) = child_scenario.child_scenario.get() else {
            continue
        };

        let mut child_scenario = state.read_tag_group::<Scenario>(child_scenario_path)
            .expect("child scenario")
            .to_owned();

        for bsp in &child_scenario.structure_bsps {
            let bsp = &bsp.structure_bsp;
            let found = scenario.structure_bsps.iter().any(|i| &i.structure_bsp == bsp);
            if !found {
                state.warn(
                    tag_path,
                    format_args!("Child scenario {child_scenario_path} contains BSP {bsp} which is not present in the main scenario. BSPs do not get merged."),
                    PostprocessWarningType::MismatchedChildScenarioBSP
                );
            }
        }

        // These cannot be merged.
        let nope = [
            &scenario.player_starting_locations as *const dyn EditableIndexedTagField,
            &scenario.child_scenarios as *const dyn EditableIndexedTagField,
            &scenario.structure_bsps as *const dyn EditableIndexedTagField,
            &scenario.skies as *const dyn EditableIndexedTagField,
            &scenario.detail_object_collection_palette as *const dyn EditableIndexedTagField,
            &scenario.scripts as *const dyn EditableIndexedTagField,
            &scenario.script_globals as *const dyn EditableIndexedTagField,
            &scenario.references as *const dyn EditableIndexedTagField,
            &scenario.source_files as *const dyn EditableIndexedTagField,
        ];

        // First, object names should be checked.
        for object_name_child in &child_scenario.object_names {
            let name = object_name_child.name;
            for object_name_main in &scenario.object_names {
                if object_name_main.name.as_str().eq_ignore_ascii_case(name.as_str()) {
                    // tool.exe literally screams at you if this happens (but keeps going anyway lol)
                    fail_postprocess!("Duplicate object name {name} found when merging scenarios.");
                }
            }
        }

        // Next, update the indices.
        for scenario_field_name in child_scenario.fields() {
            let child_scenario_field = child_scenario.get_field_mut(*scenario_field_name).expect("get_field");
            let Some(child_scenario_indexed) = child_scenario_field.get_indexed_mut() else {
                continue;
            };

            let ptr = child_scenario_indexed as *const dyn EditableIndexedTagField;
            if nope.contains(&ptr) {
                continue
            }

            fn update_reflexive_fields(scenario: &Scenario, child_scenario_indexed: &mut dyn EditableIndexedTagField) -> Result<(), PostprocessError> {
                for block in child_scenario_indexed.iter_mut() {
                    let inner_fields = block.get_composite_mut().expect("reflexive should have composite");
                    for reflexive_field in inner_fields.fields() {
                        let block_field = inner_fields.get_field_mut(*reflexive_field).expect("inner field should exist!");

                        let Some(index) = block_field.get_reflexive_index_mut() else {
                            let Some(indexed) = block_field.get_indexed_mut() else {
                                continue;
                            };
                            update_reflexive_fields(scenario, indexed)?;
                            continue;
                        };

                        let referenced_struct_name = index.get_reflexive_struct();
                        let referenced_field_name = index.get_reflexive_name();
                        if referenced_struct_name != scenario.get_field_type_name() {
                            continue;
                        }

                        let count = scenario
                            .get_field(index.get_reflexive_name())
                            .expect("reflexive name")
                            .get_indexed()
                            .expect("reflexive should be indexed")
                            .item_count();

                        let index = index.get_index_mut();
                        let added = count.saturating_add(index.0 as usize);
                        if added > (u16::MAX as usize) {
                            fail_postprocess!("Index to {referenced_struct_name}::{referenced_field_name} overflows when merging scenarios.");
                        }
                        index.0 = added as u16;
                    }
                }

                Ok(())
            }

            update_reflexive_fields(scenario, child_scenario_indexed)?;
        }

        // Next, merge!
        for scenario_field_name in child_scenario.fields() {
            let child_scenario_field = child_scenario.get_field_mut(*scenario_field_name).expect("get_field");
            let Some(child_scenario_indexed) = child_scenario_field.get_indexed() else {
                continue;
            };

            let ptr = child_scenario_indexed as *const dyn EditableIndexedTagField;
            if nope.contains(&ptr) {
                continue
            }

            let into_scenario = scenario
                .get_field_mut(scenario_field_name)
                .expect("reflexive name")
                .get_indexed_mut()
                .expect("reflexive should be indexed");

            for block in child_scenario_indexed {
                into_scenario.insert_item(into_scenario.item_count(), block).expect("merging the same type, should work");
            }
        }

        // Lastly, merge scripts. This has to be done manually to avoid duplicate source files.
        for source in child_scenario.source_files {
            if scenario.source_files.iter().any(|i| i.name == source.name) {
                continue
            }
            scenario.source_files.push(source);
        }
    }

    Ok(())
}

fn check_duplicate_object_names(scenario: &mut Scenario) -> Result<(), PostprocessError> {
    for (index, object_name) in scenario.object_names.iter().enumerate() {
        let name = object_name.name;
        for i in scenario.object_names.iter().take(index) {
            if i.name.as_str().eq_ignore_ascii_case(name.as_str()) {
                fail_postprocess!("Duplicate object name {name} detected.");
            }
        }
    }
    Ok(())
}

fn check_palettes(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let matches = get_objects_and_palettes!(scenario);

    for (spawner, palette, object_type) in matches {
        for (index, spawn) in spawner.iter().enumerate() {
            let spawn_struct = spawn.get_composite().expect("spawners should be a struct");
            let field_type = spawn_struct.get_field("type")
                .expect("no type present in spawner")
                .get_reflexive_index()
                .expect("not an index")
                .get_index()
                .index();

            match field_type {
                Some(n) => if palette.item_count() <= n {
                    fail_postprocess!("{object_type} #{index} has an out-of-bounds object index!");
                }
                None => {
                    state.warn(tag_path, format_args!("{object_type} #{index} has no type set."), PostprocessWarningType::UnusedData);
                }
            }
        }
    }

    Ok(())
}

fn get_all_bsps_for_postprocessing<'a>(scenario: &mut Scenario, action: Action, state: &'a dyn PostprocessState) -> Result<Vec<(usize, &'a ScenarioStructureBSP, &'a ModelCollisionGeometryBSP)>, PostprocessError> {
    if !action.postprocess() {
        return Ok(Vec::new())
    }

    // TODO: make sure this count gets checked
    if scenario.structure_bsps.len() > 16 {
        fail_postprocess!("Too many BSPs in the scenario tag");
    }

    let mut bsps = Vec::new();
    bsps.reserve(scenario.structure_bsps.len());

    for (index, bsp_entry) in scenario.structure_bsps.iter().enumerate() {
        let Some(bsp_tag) = bsp_entry.structure_bsp.get() else {
            fail_postprocess!("BSP #{index} is unset")
        };
        let bsp = state.read_tag_group::<ScenarioStructureBSP>(bsp_tag).expect("bsp needs to be a bsp tag");
        bsps.push((index, bsp, &bsp.collision_bsp[0]));

        for previous_bsp in &scenario.structure_bsps[..index] {
            if previous_bsp.structure_bsp == bsp_entry.structure_bsp {
                fail_postprocess!("BSP {} is duplicated", bsp_entry.structure_bsp);
            }
        }
    }

    Ok(bsps)
}

fn set_bsp_indices_for_scenery(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState, bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) {
    if !action.postprocess() {
        return
    }

    for (spawn_index, spawn) in scenario.scenery.iter_mut().enumerate() {
        let Some(scenery) = spawn._type.index().and_then(|index| scenario.scenery_palette[index].name.get()) else {
            continue
        };

        let bounding_offset = state.read_tag_group::<Object>(scenery).expect("palette needs to be an object").bounding_offset;
        let bounding_offset_bsps = generate_bsp_spawn_index_bitfield(spawn.placement.position, spawn.placement.rotation, bounding_offset, bsps);
        let base_bsps = generate_bsp_spawn_index_bitfield(spawn.placement.position, spawn.placement.rotation, Vector3D::ZEROED, bsps);
        spawn.bsp_indices = base_bsps | bounding_offset_bsps;

        if spawn.bsp_indices == 0 {
            state.warn(tag_path, format_args!("Scenery spawn #{spawn_index} is not inside any BSPs. It won't spawn."), PostprocessWarningType::MisplacedObjects);
            return;
        }

        if bounding_offset_bsps & base_bsps != base_bsps {
            state.warn(
                tag_path,
                format_args!("Scenery spawn #{spawn_index} has its spawn point outside of the BSP but its bounding offset pushes it inside. The object will spawn, but it may have lighting bugs."),
                PostprocessWarningType::BoundingOffsetInsideBSP
            );
        }
    }
}

fn generate_bsp_spawn_index_bitfield(point: Vector3D, rotation: Euler3D, bounding_offset: Vector3D, bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) -> u16 {
    let mut transformation = Matrix4x3::from(rotation);
    transformation.position = point;
    let point_to_check = transformation.transform_point(bounding_offset);

    let mut spawning_bsps = 0;
    for (index, _bsp, collision) in bsps.iter().copied() {
        let inside_bsp = (collision.point_inside_bsp(&point_to_check).expect("checked bsp") as u16) << index;
        spawning_bsps |= inside_bsp;
    }
    spawning_bsps
}

fn compile_scripts(scenario: &mut Scenario, action: Action, _tag_path: &TagPath, _state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    if (!scenario.scripts.is_empty() || !scenario.script_globals.is_empty() || !scenario.references.is_empty()) && scenario.source_files.is_empty() {
        fail_postprocess!("No source files are present, but script data is defined. You need to recompile the scripts.")
    }

    todo!("compile_scripts")
}


fn check_player_spawns(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) {
    // TODO: Check if spawns are inside the BSP

    if !action.postprocess() {
        return
    }

    match scenario._type {
        ScenarioType::UserInterface => { /* UI maps don't have any player spawning requirements */ },
        ScenarioType::Singleplayer => {
            let valid_spawns = scenario.player_starting_locations
                .iter()
                .any(|i| [i.type_0, i.type_1, i.type_2, i.type_3]
                    .iter()
                    .all(|i| *i == ScenarioSpawnType::None)
                );
            if !valid_spawns {
                state.warn(tag_path, format_args!("No singleplayer player spawns present (must have at least one spawn with all types set to 'none')."), PostprocessWarningType::NoUsablePlayerSpawns);
            }
        },
        ScenarioType::Multiplayer => {
            let mut ctf_teams = BTreeMap::<u16, usize>::new();

            let mut slayer_spawns_total = 0usize;
            let mut race_spawns_total = 0usize;
            let mut king_spawns_total = 0usize;
            let mut oddball_spawns_total = 0usize;
            let mut viable_spawns = false;

            for i in &scenario.player_starting_locations {
                let mut ctf = false;
                let mut slayer = false;
                let mut oddball = false;
                let mut king = false;
                let mut race = false;

                for s in [i.type_0, i.type_1, i.type_2, i.type_3] {
                    match s {
                        ScenarioSpawnType::None => continue,
                        ScenarioSpawnType::Ctf => {
                            ctf = true;
                        }
                        ScenarioSpawnType::Slayer => {
                            slayer = true;
                        }
                        ScenarioSpawnType::Oddball => {
                            oddball = true;
                        }
                        ScenarioSpawnType::KingOfTheHill => {
                            king = true;
                        }
                        ScenarioSpawnType::Race => {
                            race = true;
                        }
                        ScenarioSpawnType::Terminator => continue,
                        ScenarioSpawnType::AllGames => {
                            ctf = true;
                            slayer = true;
                            oddball = true;
                            king = true;
                            race = true;
                            break;
                        }
                        ScenarioSpawnType::AllExceptCtf => {
                            slayer = true;
                            oddball = true;
                            king = true;
                            race = true;
                        }
                        ScenarioSpawnType::AllExceptRaceAndCtf => {
                            slayer = true;
                            oddball = true;
                            king = true;
                        }
                    }
                }

                if !ctf && !slayer && !oddball && !race && !king {
                    continue
                }

                viable_spawns = true;

                if oddball {
                    oddball_spawns_total += 1;
                }

                if slayer {
                    slayer_spawns_total += 1;
                }

                if race {
                    race_spawns_total += 1;
                }

                if king {
                    king_spawns_total += 1;
                }

                if ctf {
                    if let Some(count) = ctf_teams.get_mut(&i.team_index.0) {
                        *count += 1;
                    }
                    else {
                        ctf_teams.insert(i.team_index.0, 1);
                    }
                }
            }

            if !viable_spawns {
                state.warn(tag_path, format_args!("No multiplayer player spawns are present in the map."), PostprocessWarningType::NoUsablePlayerSpawns);
                return
            }

            if ctf_teams.keys().any(|i| *i == 0xFFFF) {
                state.warn(tag_path, format_args!("CTF player spawn with team index None defined (did you forget to set the team?)"), PostprocessWarningType::UnusedData);
            }

            let complain = |gametype: &str, amount: usize| {
                if amount < DEFAULT_MAX_NUMBER_PLAYERS {
                    if amount == 0 {
                        // Separate warning type as you might want to mute this warning if you don't want your map to support a given gametype.
                        state.warn(tag_path, format_args!("No {gametype} spawns defined."), PostprocessWarningType::MissingGametypePlayerSpawns);
                    } else {
                        state.warn(tag_path, format_args!("Only {amount} {gametype} spawn(s) defined, where the maximum number of players is {DEFAULT_MAX_NUMBER_PLAYERS}."), PostprocessWarningType::InsufficientPlayerSpawnCount);
                    }
                }
            };

            complain("slayer", slayer_spawns_total);
            complain("oddball", oddball_spawns_total);
            complain("king of the hill", king_spawns_total);
            complain("race", race_spawns_total);

            if ctf_teams.is_empty() {
                state.warn(tag_path, format_args!("No CTF spawns defined."), PostprocessWarningType::MissingGametypePlayerSpawns);
            }
            else {
                let complain_team = |team: &str, amount: usize| {
                    if amount < DEFAULT_MAX_NUMBER_PLAYERS {
                        if amount == 0 {
                            // Separate warning type as you might want to mute this warning if you don't want your map to support a given gametype.
                            state.warn(tag_path, format_args!("No {team} spawns defined."), PostprocessWarningType::MissingGametypePlayerSpawns);
                        } else {
                            state.warn(tag_path, format_args!("Only {amount} {team} spawn(s) defined, where the maximum number of players is {DEFAULT_MAX_NUMBER_PLAYERS}. NOTE: You should expect games where all players in the game are on the same team."), PostprocessWarningType::InsufficientPlayerSpawnCount);
                        }
                    }
                };

                complain_team("red team (#0)", *ctf_teams.get(&0).unwrap_or(&0));
                complain_team("blue team (#1)", *ctf_teams.get(&1).unwrap_or(&0));

                // any non-standard teams can be complained about too
                for (&team_index, &count) in &ctf_teams {
                    if team_index == 0 || team_index == 1 || team_index == 0xFFFF {
                        continue
                    }

                    let team_name = alloc::format!("custom team (#{team_index})");
                    complain_team(&team_name, count);
                }
            }
        }
    }
}
