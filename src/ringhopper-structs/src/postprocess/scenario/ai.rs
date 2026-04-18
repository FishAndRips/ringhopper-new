use alloc::string::String;
use funnel_web::collision_bsp::{CollisionBSPError, CollisionBSPFunctions, CollisionBSPTestVectorFlags};
use funnel_web::id::Index;
use funnel_web::vector::Vector3D;
use tinyvec::ArrayVec;
use crate::definitions::tag::scenario::{Scenario, ScenarioSquadAttacking};
use crate::postprocess::Action;
use crate::{Parameters, PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use crate::definitions::tag::model_collision_geometry::ModelCollisionGeometryBSP;
use crate::definitions::tag::scenario_structure_bsp::ScenarioStructureBSP;
use crate::postprocess::scenario::MAX_BSPS;

pub fn postprocess_command_list(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState, all_bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for (command_list_index, command_list) in scenario.command_lists.iter_mut().enumerate() {
        if command_list.flags.manual_bsp_index {
            command_list.precomputed_bsp_index = command_list.manual_bsp_index.0;
            continue
        }

        let best_bsp = find_best_bsp(
            || command_list.points.iter().map(|p| p.position),
            all_bsps,
            ground_point_valid_for_bsp
        )?;

        match best_bsp {
            Some(n) => {
                command_list.precomputed_bsp_index = Index::from_usize(n.first_bsp_index).expect("bsp within count???");
                if n.first_bsp_count < n.total {
                    let mut further_note = String::new();

                    if n.first_bsp_count != n.best_bsp_count {
                        let best_bsp_index = n.best_bsp_index;
                        let best_bsp_count = n.best_bsp_count;
                        further_note = alloc::format!(" (NOTE: BSP#{best_bsp_index} has {best_bsp_count} valid ground points; use manual BSP indices to select it)")
                    }

                    let points = n.first_bsp_count;
                    let total_points = n.total;
                    let bsp = n.first_bsp_index;
                    state.warn(tag_path, format_args!("Command list #{command_list_index} is only partially inside BSP#{bsp} ({points} / {total_points} ground points){further_note}"), PostprocessWarningType::MisplacedCommandLists);
                }
            },
            None => {
                command_list.precomputed_bsp_index = Index::new();
                state.warn(tag_path, format_args!("Command list #{command_list_index} is not inside any BSP (no valid ground points for any BSP)"), PostprocessWarningType::MisplacedCommandLists);
            }
        }
    }

    for command_list in &mut scenario.command_lists {
        let Some(bsp) = command_list.precomputed_bsp_index.index().map(|i| all_bsps[i]) else { continue };
        for point in &mut command_list.points {
            point.surface_index = surface_index_from_ground_point(bsp.2, point.position)?.unwrap_or(usize::MAX) as u32;
        }
    }

    Ok(())
}

struct BestBSP {
    first_bsp_index: usize,
    first_bsp_count: usize,
    best_bsp_index: usize,
    best_bsp_count: usize,
    total: usize
}

fn find_best_bsp<I: Iterator<Item = Vector3D>, F: FnMut() -> I, B: FnMut(&ModelCollisionGeometryBSP, Vector3D) -> Result<bool, CollisionBSPError>>(
    mut points: F,
    all_bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)],
    mut test_function: B
) -> Result<Option<BestBSP>, CollisionBSPError> {
    let mut possible_bsps = ArrayVec::<[(usize, usize); MAX_BSPS]>::new();
    let total = points().count();

    for bsp in all_bsps {
        let mut points_found = 0;
        for point in points() {
            if test_function(bsp.2, point)? {
                points_found += 1;
            }
        }

        if points_found > 0 {
            possible_bsps.push((bsp.0, points_found));
        }
    }

    let Some(&(bsp, points)) = possible_bsps.first() else { return Ok(None) };

    if points < total {
        let &(best_bsp_index, best_bsp_count) = possible_bsps
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .expect("should have a best BSP");

        Ok(Some(BestBSP {
            first_bsp_index: bsp,
            first_bsp_count: points,
            best_bsp_index,
            best_bsp_count,
            total
        }))
    }
    else {
        Ok(Some(BestBSP {
            first_bsp_index: bsp,
            first_bsp_count: points,
            best_bsp_index: bsp,
            best_bsp_count: points,
            total
        }))
    }
}

pub fn postprocess_encounters(scenario: &mut Scenario, action: Action, tag_path: &TagPath, state: &dyn PostprocessState, all_bsps: &[(usize, &ScenarioStructureBSP, &ModelCollisionGeometryBSP)]) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for (encounter_index, encounter) in &mut scenario.encounters.iter_mut().enumerate() {
        for squad in &mut encounter.squads {
            for move_position in &mut squad.move_positions {
                if move_position.weight <= 0.0 {
                    move_position.weight = 0.001
                }
            }

            if squad.attacking.as_int(Parameters::CACHE_FILES) == 0 {
                squad.attacking = ScenarioSquadAttacking::from_int(u32::MAX, Parameters::TAG_FILES);
            }

            let mut order = [
                &mut squad.attacking,
                &mut squad.attacking_search,
                &mut squad.attacking_guard,
                &mut squad.defending,
                &mut squad.defending_search,
                &mut squad.defending_guard,
                &mut squad.pursuing
            ];

            for i in 1..order.len() - 1 {
                let (a,b) = order.split_at_mut(i);

                let a = a.last_mut().expect("bad set_surface_indices_for_ai loop a");
                let b = b.first_mut().expect("bad set_surface_indices_for_ai loop b");

                if b.as_int(Parameters::CACHE_FILES) == 0 {
                    **b = **a;
                }
            }

            squad.attacking_search = ScenarioSquadAttacking::from_int(squad.attacking_search.as_int(Parameters::CACHE_FILES) | squad.attacking.as_int(Parameters::CACHE_FILES), Parameters::CACHE_FILES);
            squad.defending_search = ScenarioSquadAttacking::from_int(squad.defending_search.as_int(Parameters::CACHE_FILES) | squad.defending.as_int(Parameters::CACHE_FILES), Parameters::CACHE_FILES);
        }

        if encounter.flags.manual_bsp_index_specified {
            encounter.precomputed_bsp_index = encounter.manual_bsp_index.0;
        }
        else {
            let test_function = match encounter.flags._3d_firing_positions {
                true => cluster_index_valid_for_bsp,
                false => ground_point_valid_for_bsp
            };

            let what_is_being_tested;
            let best_bsp;

            if encounter.firing_positions.is_empty() {
                best_bsp = find_best_bsp(
                    || encounter.squads.iter().map(|s| s.starting_locations.iter().map(|l| l.position)).flatten(),
                    all_bsps,
                    test_function
                )?;
                what_is_being_tested = "starting locations";
            }
            else {
                best_bsp = find_best_bsp(
                    || encounter.firing_positions.iter().map(|p| p.position),
                    all_bsps,
                    test_function
                )?;
                what_is_being_tested = "firing positions";
            }

            match best_bsp {
                Some(n) => {
                    encounter.precomputed_bsp_index = Index::from_usize(n.first_bsp_index).expect("bsp within count???");
                    if n.first_bsp_count < n.total {
                        let mut further_note = String::new();

                        if n.first_bsp_count != n.best_bsp_count {
                            let best_bsp_index = n.best_bsp_index;
                            let best_bsp_count = n.best_bsp_count;
                            further_note = alloc::format!(" (NOTE: BSP#{best_bsp_index} has {best_bsp_count} valid {what_is_being_tested}; use manual BSP indices to select it)")
                        }

                        let points = n.first_bsp_count;
                        let total_points = n.total;
                        let bsp = n.first_bsp_index;
                        state.warn(tag_path, format_args!("Encounter #{encounter_index} is only partially inside BSP#{bsp} ({points} / {total_points} {what_is_being_tested}){further_note}"), PostprocessWarningType::MisplacedCommandLists);
                    }
                },
                None => {
                    encounter.precomputed_bsp_index = Index::new();
                    state.warn(tag_path, format_args!("Encounter #{encounter_index} is not inside any BSP (no valid {what_is_being_tested} for any BSP)"), PostprocessWarningType::MisplacedCommandLists);
                }
            }
        }
    }

    for encounter in &mut scenario.encounters {
        let Some(bsp) = encounter.precomputed_bsp_index.index().map(|i| all_bsps[i]) else { continue };
        for firing_position in &mut encounter.firing_positions {
            if !encounter.flags._3d_firing_positions {
                firing_position.surface_index = surface_index_from_ground_point(bsp.2, firing_position.position)?.unwrap_or(usize::MAX) as u32;
            }
            firing_position.cluster_index = get_cluster_index(bsp.2, firing_position.position)?.and_then(Index::from_usize).unwrap_or(Index::new());
        }
        for squad in &mut encounter.squads {
            for move_position in &mut squad.move_positions {
                if !encounter.flags._3d_firing_positions {
                    move_position.surface_index = surface_index_from_ground_point(bsp.2, move_position.position)?.unwrap_or(usize::MAX) as u32;
                }
                move_position.cluster_index = get_cluster_index(bsp.2, move_position.position)?.and_then(Index::from_usize).unwrap_or(Index::new());
            }
            for starting_position in &mut squad.starting_locations {
                starting_position.cluster_index = get_cluster_index(bsp.2, starting_position.position)?.and_then(Index::from_usize).unwrap_or(Index::new());
            }
        }
    }

    Ok(())
}


/// Find the surface index given a point very close to the ground.
fn surface_index_from_ground_point(bsp: &ModelCollisionGeometryBSP, ground_point: Vector3D) -> Result<Option<usize>, CollisionBSPError> {
    let origin = ground_point + Vector3D { x: 0.0, y: 0.0, z: 0.5 };
    let down = Vector3D { x: 0.0, y: 0.0, z: -1.0 };
    let check = bsp.test_vector(
        CollisionBSPTestVectorFlags {
            test_front_facing_surfaces: true,
            ..CollisionBSPTestVectorFlags::default()
        },
        &[],
        origin,
        down,
        f32::MAX,
    )?;
    Ok(check.hit_surface.map(|i| i.surface_index))
}

fn ground_point_valid_for_bsp(bsp: &ModelCollisionGeometryBSP, point: Vector3D) -> Result<bool, CollisionBSPError> {
    surface_index_from_ground_point(bsp, point).map(|i | i.is_some())
}

fn cluster_index_valid_for_bsp(bsp: &ModelCollisionGeometryBSP, point: Vector3D) -> Result<bool, CollisionBSPError> {
    get_cluster_index(bsp, point).map(|i| i.is_some())
}

fn get_cluster_index(bsp: &ModelCollisionGeometryBSP, point: Vector3D) -> Result<Option<usize>, CollisionBSPError> {
    bsp.leaf_index_for_point_3d(Vector3D {
        x: point.x,
        y: point.y,
        z: point.z + 0.1
    })
}
