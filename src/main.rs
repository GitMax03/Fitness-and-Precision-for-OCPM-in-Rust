use crate::structs_for_ocpm::{Marking, OCPN, hash_marking};
use crate::utils::{export_ocel_to_jsonocel_file, is_superset, ocel_preprocessing, ocpn_preprocessing, print_contexts, clean_ocel_ids_and_types_for_json, analyze_ocel_folder, get_ocpn_path, clean_ocel, filter_ocel, auto_eval, apply_filter, export_filtered_ocel, analyze_runs, analyize_ocpn, print_multiplicities};


use rustworkx_core::{petgraph};


use crate::generator::{generate};

use counter::Counter;
use process_mining::OCEL;
use process_mining::ocel::ocel_struct::OCELEvent;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::RwLock;
use std::task::Context;



use crate::MDFA::{abstract_marking_with_map, get_all_rename_maps};
use ahash::{AHashMap, AHashSet, AHasher};

mod enabled_log_act;
mod enabled_model_act;
mod evaluator;
mod structs_for_ocpm;
mod test;
mod utils;

mod MDFA;
mod generator;

//better allocator for performance
extern crate mimalloc;
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const NUMBER_OF_ITERATIONS: u32 = 1;
const VERSION_NAME: &str = "v2_multi-threaded +";
const LOG_MODEL: &str = "small running example";

const SAVE:bool = true;
/*
100.000 results (no low energy mode) v1:small running example:
0.00087 55%
0.00040 25.7%
0.00020 13.0%
0.000073 4.6%
0.000020 1.2%
0.00000067 0.04%
average: 0.001576
 */

/*
not replayble events: in current model:
    702, 703, 704, 705, 706, 707 ("Refuse offer" without "Deny")


 */

use once_cell::sync::Lazy;
use petgraph::dot::{Config, Dot};
use crate::enabled_log_act::construct_event_object_graph;
use crate::test::{get_test_ocel, get_test_ocpn_small};

fn main() {
    //clean_ocel_ids_and_types_for_json("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/OCEL/temp/sap_o2c.jsonocel", "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/OCEL/temp/sap_o2c.jsonocel");
    print_multiplicities("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/temp");
    return;
    
}

fn get_ocel_ocpn() -> (OCEL, OCPN, Option<String>) {
    //1 runnung example, 3: changing:intermediate, 4: full changing
    match 5 {
        1 => {
            // Load OCEL and OCPN from files or create them
            let ocel = test::get_test_ocel();
            let ocpn = test::get_test_ocpn_small();
            (ocel, ocpn, Option::from("running example".to_string()))
        }
        2 => {
            // Load OCEL and OCPN from files or create them
            let ocel = utils::import_ocel_from_jsonocel(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/intermediate.jsonocel",
            );
            let ocpn = utils::import_from_json(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/ocpn_intermediate.json",
            );
            (ocel, ocpn, None)
        }
        3 => {
            let ocel = utils::import_ocel_from_jsonocel(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/intermediate.jsonocel",
            );
            let ocpn = utils::import_from_json(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/ocpn_intermediate_changing.json",
            );
            (ocel, ocpn, Option::from("intermediate_changing".to_string()))
        }
        4 => {
            let ocel = utils::import_ocel_from_jsonocel(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/changing_ocel.jsonocel",
            );
            let ocpn = utils::import_from_json(
                "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/ocpn_intermediate_changing.json",
            );
            (ocel, ocpn, Option::from("changing_ocel".to_string()))
        }
        5 => {


            let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCEL/sap_o2c.jsonocel";

            //let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/OCEL/strict_filter_SAP_P2P.jsonocel";

            //let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_evaluation/OCEL/intermediate.jsonocel";
            //let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_evaluation/OCEL/OM_2014Events.json";
            //let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_evaluation/OCEL/p2p_2010Events.json";
            let ocpn_path = get_ocpn_path(ocel_path, "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCPN");

            //clean_ocel_ids_and_types_for_json(ocel_path, ocel_path);
            let mut ocel = utils::import_ocel(ocel_path);



            let ocpn = ocpn_preprocessing(utils::import_from_json(
                ocpn_path.as_str(),
            ));
            //ocpn.export_dot_to_file("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/vis.dot".to_string());
            

            //ocel = filter_ocel(ocel, ocpn.object_to_type.clone(), 5, 15, 80);
            //process_mining::export_ocel_xml_path(&ocel.clone(), "/Users/maxbaumeister/PycharmProjects/ocpa_2/sample_logs/ocel2/xmlocel/BA_Eval/ocel2-export_rs.xml").expect("export failed");
            //export_ocel_to_jsonocel_file(&ocel.clone(), "/Users/maxbaumeister/PycharmProjects/ocpa_2/sample_logs/ocel2/xmlocel/BA_Eval/ocel-export_rs.jsonocel").expect("export failed");


            (ocel, ocpn, None)

        }
        6 => {
            let ocel_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_evaluation/OCEL/OCEL2/ocel2-export.xml";

            let ocpn_path = get_ocpn_path(ocel_path, "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_evaluation/OCPN");

            //let mut ocel = utils::import_ocel(ocel_path);
            let mut ocel = process_mining::import_ocel_xml_file(ocel_path);
            ocel = clean_ocel(ocel);



            //export OCEL to python
            //process_mining::export_ocel_xml_path(&ocel.clone(), "/Users/maxbaumeister/PycharmProjects/ocpa_2/sample_logs/ocel2/xmlocel/BA_Eval/ocel2-export_rs.xml").expect("export failed");
            //println!("Exported");
            let ocpn = ocpn_preprocessing(utils::import_from_json(
                ocpn_path.as_str(),
            ));
            //ocpn.export_dot_to_file("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/vis.dot".to_string());


            ocpn.export_dot_to_file("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/vis.dot".parse().unwrap());

            ocel = filter_ocel(ocel, ocpn.object_to_type.clone(), 0, 15, -1);

            (ocel, ocpn, Option::from(ocel_path.to_string()))
        }

        _ => {
            // Load OCEL and OCPN from files or create them
            let ocel = test::get_test_ocel();
            let ocpn = test::get_test_ocpn_small();
            (ocel, ocpn, Option::from("running example".to_string()))
        }
    }
}


fn run() {
    let (ocel, ocpn, ocel_path) = get_ocel_ocpn();
    
    let path:Option<&str> = match &ocel_path {
        Some(p) => Option::from(p.as_str()),
        None => None,
    };

    let (fitness, precision) = evaluator::apply(ocel, ocpn.clone(), path);
    println!("\n-----------------------------------------");
    println!("fitness: {}", fitness);
    println!("precision: {}", precision);
    println!("-----------------------------------------\n");
}

fn test_running_example() {
    let ocel = ocel_preprocessing(test::get_test_ocel());
    let ocpn = ocpn_preprocessing(test::get_test_ocpn_small());
    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();

    //test
    print!("#################### test running example ####################\n");

    let (presets, id_presets) = enabled_log_act::get_event_presets(&ocel, &id_to_event);
    println!(".");

    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let all_object_types: HashSet<String> =
        ocel.object_types.iter().map(|ot| ot.name.clone()).collect();

    let (contexts, bindings) = enabled_log_act::_get_contexts_and_bindings(
        &ocel,
        &id_to_event,
        &None,
        &all_object_types,
        &ocpn.object_to_type,
    );

    //let (contexts, bindings) = enabled_log_act::get_contexts_and_bindings(&ocel,&presets);
    println!(".");
    //print_contexts(&contexts);

    let (enabled_log_activities, context_map) =
        enabled_log_act::get_enabled_log_activities(&contexts, &id_to_event);
    println!(".");
    let enabled_model_activities = enabled_model_act::get_enabled_model_activities_(
        &ocpn,
        &bindings,
        &contexts,
        &context_map,
        &all_object_types,
        &id_to_event,
        &id_presets,
    );
    println!(".");
    /*
    save_to_ron(&presets.clone(), "src/test/Expected results (running example)/presets.ron").unwrap();
    save_to_ron(&bindings.clone(), "src/test/Expected results (running example)/bindings.ron").unwrap();
    save_to_ron(&contexts.clone(), "src/test/Expected results (running example)/contexts.ron").unwrap();
    save_to_ron(&context_map.clone(), "src/test/Expected results (running example)/context_map.ron").unwrap();
    save_to_ron(&enabled_log_activities.clone(), "src/test/Expected results (running example)/enabled_log_activities.ron").unwrap();
    save_to_ron(&enabled_model_activities.clone(), "src/test/Expected results (running example)/enabled_model_activities.ron").unwrap();

     */

    test_presets(&presets);
    test_bindings(&bindings);
    test_contexts(&contexts);
    test_context_map(&context_map);

    test_enabled_log_activities(&enabled_log_activities, ocpn.clone());
    test_enabled_model_activities(
        &enabled_model_activities
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect(),
        ocpn.clone(),
    );

    let (fitness, precision) = evaluator::apply(ocel, ocpn.clone(), None);

    println!("fitness: {}", fitness);
    println!("precision: {}", precision);

    assert_eq!(fitness, 1.0);
    assert_eq!((precision * 100.0).round() / 100.0, 0.89);
}

pub fn save_to_ron<T: Serialize>(data: &T, path: &str) -> std::io::Result<()> {
    // create directory if needed
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let ron_string = ron::ser::to_string_pretty(data, PrettyConfig::default())
        .expect("Failed to serialize to RON");
    let mut file = File::create(path)?;
    file.write_all(ron_string.as_bytes())?;
    Ok(())
}
pub fn load_from_ron<T: for<'de> Deserialize<'de>>(path: &str) -> std::io::Result<T> {
    let content = fs::read_to_string(path)?;
    let data: T = ron::from_str(&content).expect("Failed to deserialize from RON");
    Ok(data)
}

pub fn test_context_map(context_map: &HashMap<u64, HashSet<String>>) {
    println!("#################### test context_map ####################");

    // Erwartete Map: gleiche Struktur wie `context_map`
    let expected: HashMap<u64, HashSet<String>> =
        load_from_ron("src/test/Expected results (running example)/context_map.ron").unwrap();

    // Größe prüfen
    assert_eq!(
        context_map.len(),
        expected.len(),
        "context_map.len (expected: {}, actual: {})",
        expected.len(),
        context_map.len()
    );

    // Inhalte vergleichen
    for (key, actual_set) in context_map {
        assert!(
            expected.contains_key(key),
            "Key '{}' not found in expected context_map",
            key
        );

        let expected_set = &expected[key];

        assert_eq!(
            actual_set, expected_set,
            "Set for key '{}' differs (expected: {:?}, actual: {:?})",
            key, expected_set, actual_set
        );
    }

    println!("   -> context_map is correct!\n");
}
fn test_enabled_log_activities(enabled_activities: &HashMap<String, HashSet<String>>, ocpn: OCPN) {
    test_enabled_activities(enabled_activities, true, ocpn);
}
fn test_enabled_model_activities(
    enabled_activities: &HashMap<String, HashSet<String>>,
    ocpn: OCPN,
) {
    test_enabled_activities(enabled_activities, false, ocpn);
}
fn test_enabled_activities(
    enabled_activities: &HashMap<String, HashSet<String>>,
    log: bool,
    ocpn: OCPN,
) {
    println!("#################### test enabled_activities ####################");
    if log {
        println!("Log")
    } else {
        println!("Model")
    };

    let expected: HashMap<String, HashSet<String>> = if log {
        load_from_ron("src/test/Expected results (running example)/enabled_log_activities.ron")
            .unwrap()
    } else {
        load_from_ron("src/test/Expected results (running example)/enabled_model_activities.ron")
            .unwrap()
    };

    assert_eq!(
        enabled_activities.len(),
        expected.len(),
        "enabled_activities.len (expected: {}, actual: {})",
        expected.len(),
        enabled_activities.len()
    );

    for (key, actual_set) in enabled_activities {
        assert!(
            expected.contains_key(key),
            "Key '{}' not found in expected enabled_activities.",
            key
        );

        let expected_set = &expected[key];
        assert_eq!(
            actual_set, expected_set,
            "Sets for key '{}' differ (expected: {:?}, actual: {:?})",
            key, expected_set, actual_set
        );
    }

    println!("   -> enabled_activities are correct!\n\n\n");
}

fn test_presets(presets: &HashMap<String, Vec<OCELEvent>>) {
    println!("#################### test presets ####################");

    // Load expected presets from file
    let expected: HashMap<String, Vec<OCELEvent>> =
        load_from_ron("src/test/Expected results (running example)/presets.ron").unwrap();

    // Compare map sizes
    assert_eq!(
        presets.len(),
        expected.len(),
        "presets.len (expected: {}, actual: {})",
        expected.len(),
        presets.len()
    );

    // Compare entries
    for (key, actual_vec) in presets {
        assert!(
            expected.contains_key(key),
            "Key '{}' not found in expected presets",
            key
        );

        let expected_vec = &expected[key];

        // Compare lengths
        assert_eq!(
            actual_vec.len(),
            expected_vec.len(),
            "Vector for key '{}' has wrong length (expected {}, actual {})",
            key,
            expected_vec.len(),
            actual_vec.len()
        );

        // Compare vectors as sets to ignore order
        let actual_set: HashSet<_> = actual_vec.iter().map(|event| event.id.clone()).collect();
        let expected_set: HashSet<_> = expected_vec.iter().map(|event| event.id.clone()).collect();

        assert_eq!(
            actual_set, expected_set,
            "Values for key '{}' differ (expected: {:?}, actual: {:?})",
            key, expected_vec, actual_vec
        );
    }

    println!("   -> Presets are correct!\n\n\n");
}

fn test_contexts(contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>) {
    // Test
    println!("#################### test contexts ####################");

    // Load expected contexts of running example
    let expected: HashMap<String, HashMap<String, Counter<Vec<String>>>> =
        load_from_ron("src/test/Expected results (running example)/contexts.ron").unwrap();

    // Compare number of context entries (event_ids)
    assert_eq!(
        contexts.len(),
        expected.len(),
        "contexts.len (expected: {}, actual: {})",
        expected.len(),
        contexts.len()
    );

    // Compare each event_id
    for (event_id, actual_otypes) in contexts {
        assert!(
            expected.contains_key(event_id),
            "Event ID '{}' not found",
            event_id
        );

        let expected_otypes = &expected[event_id];

        // Compare number of object types
        assert_eq!(
            actual_otypes.len(),
            expected_otypes.len(),
            "Object types for event '{}' have different length",
            event_id
        );

        for (otype, actual_counter) in actual_otypes {
            assert!(
                expected_otypes.contains_key(otype),
                "Object type '{}' for event '{}' not found",
                otype,
                event_id
            );

            let expected_counter = &expected_otypes[otype];

            // Compare number of different activity sequences
            assert_eq!(
                actual_counter.len(),
                expected_counter.len(),
                "Number of activity sequences for event '{}' and object type '{}' differs",
                event_id,
                otype
            );

            // Compare counts and sequences (order-independent)
            for (actual_vec, actual_count) in actual_counter.iter() {
                let mut matched = false;

                for (expected_vec, expected_count) in expected_counter.iter() {
                    let set_actual: HashSet<_> = actual_vec.iter().collect();
                    let set_expected: HashSet<_> = expected_vec.iter().collect();

                    if set_actual == set_expected {
                        assert_eq!(
                            actual_count, expected_count,
                            "Count mismatch for event '{}' → object type '{}' → {:?}",
                            event_id, otype, actual_vec
                        );
                        matched = true;
                        break;
                    }
                }

                assert!(
                    matched,
                    "Activity sequence {:?} (count {}) not found in expected for event '{}' and object type '{}'",
                    actual_vec, actual_count, event_id, otype
                );
            }
        }
    }

    // All good
    println!("   -> Contexts are correct!\n\n\n");
}

pub fn test_bindings(bindings: &HashMap<String, Vec<(String, HashMap<String, HashSet<String>>)>>) {
    println!("#################### test bindings ####################");
    //laod expected bindings of running example
    let expected: HashMap<String, Vec<(String, HashMap<String, HashSet<String>>)>> =
        load_from_ron("src/test/Expected results (running example)/bindings.ron").unwrap();

    assert_eq!(
        bindings.len(),
        expected.len(),
        "bindings.len (expected: {}, actual: {})",
        expected.len(),
        bindings.len()
    );

    for (key, actual_vec) in bindings {
        let Some(expected_vec) = expected.get(key) else {
            panic!("Key {:?} not found in expected", key);
        };

        assert_eq!(
            actual_vec.len(),
            expected_vec.len(),
            "Length mismatch in inner vec for key {:?}",
            key
        );

        for (actual_prefix, actual_map) in actual_vec {
            let mut found_match = false;

            for (expected_prefix, expected_map) in expected_vec {
                if actual_prefix != expected_prefix {
                    continue;
                }

                if actual_map.len() != expected_map.len() {
                    continue;
                }

                let mut all_match = true;
                for (k, actual_set) in actual_map {
                    match expected_map.get(k) {
                        Some(expected_set) => {
                            if actual_set != expected_set {
                                all_match = false;
                                break;
                            }
                        }
                        None => {
                            all_match = false;
                            break;
                        }
                    }
                }

                if all_match {
                    found_match = true;
                    break;
                }
            }

            assert!(
                found_match,
                "No matching entry found for prefix {:?} in key {:?}",
                actual_prefix, key
            );
        }
    }

    println!("   -> Bindings are correct!\n");
}
