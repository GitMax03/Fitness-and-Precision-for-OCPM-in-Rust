

use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::task::Context;
use counter::Counter;
use process_mining::OCEL;
use process_mining::ocel::ocel_struct::OCELEvent;
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};
use crate::structs_for_ocpm::OCPN;
use crate::test::{test_enabled_model_act, test_ocpn};
use crate::utils::{is_superset, print_contexts};

mod enabled_log_act;
mod evaluator;
mod structs_for_ocpm;
mod test;
mod utils;
mod enabled_model_act;

const NUMBER_OF_ITERATIONS: u32 = 1_000;
/*
100.000 results (no low energy mode):
0.00087 55%
0.00040 25.7%
0.00020 13.0%
0.000073 4.6%
0.000020 1.2%
0.00000067 0.04%
average: 0.001576
 */

fn main() {
    
    
    //test_running_example();

    test::test_ocpn()
    
    /*
    test::test_enabled_model_act();
    
    test::test_enabled_log_activities();
    test::test_context_and_bindings();
    test::test_event_presets();
    test::test_binding_sequence();
    //test::test_eog();
    
     */
}



fn test_running_example(){

    let ocel = test::get_test_ocel();
    let ocpn = test::get_test_ocpn_small();

    

    //test
    println!("test enabled model activities for event e5");

    //assert_eq!(enabled_model_activities.len(), 2);
    //assert_eq!(enabled_model_activities,
        //["Lift off".to_string(), "Pick up @ dest".to_string()].iter().cloned().collect::<HashSet<String>>());

    let (contexts, bindings) = enabled_log_act::get_contexts_and_bindings(&ocel);
    //print_contexts(&contexts);
    let presets = enabled_log_act::get_event_presets(&ocel);
    let (enabled_log_activities, contexts_map) = enabled_log_act::get_enabled_log_activities(&ocel, &contexts);
    let enabled_model_activities = enabled_model_act::get_enabled_model_activities(&ocpn, &presets, &bindings, &contexts_map);

    //save_to_ron(&contexts_map.clone(), "src/test/Expected results (running example)/context_map.ron").unwrap();

    test_bidnings(&bindings);
    test_contexts(&contexts);
    test_presets(&presets);
    test_context_map(&contexts_map);

    test_enabled_log_activities(&enabled_log_activities, ocpn.clone());
    test_enabled_model_activities(&enabled_model_activities, ocpn.clone());

    let (fitness, precision) = evaluator::apply(ocel, ocpn.clone());

    println!("fitness: {}", fitness);
    println!("precision: {}", precision);

    assert_eq!(fitness,1.0);
    assert_eq!((precision * 100.0).round() / 100.0,0.89);

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




fn test_context_map(context_map: &HashMap<u64, (HashSet<String>, HashSet<String>)>) {
    println!("#################### test context_map ####################");

    let expected: HashMap<u64, (HashSet<String>, HashSet<String>)> =
        load_from_ron("src/test/Expected results (running example)/context_map.ron").unwrap();

    assert_eq!(
        context_map.len(),
        expected.len(),
        "context_map.len (expected: {}, actual: {})",
        expected.len(),
        context_map.len()
    );

    for (key, (actual_set1, actual_set2)) in context_map {
        assert!(
            expected.contains_key(key),
            "Key '{}' not found in expected context_map",
            key
        );

        let (expected_set1, expected_set2) = &expected[key];

        assert_eq!(
            actual_set1, expected_set1,
            "First set for key '{}' differs (expected: {:?}, actual: {:?})",
            key, expected_set1, actual_set1
        );

        assert_eq!(
            actual_set2, expected_set2,
            "Second set for key '{}' differs (expected: {:?}, actual: {:?})",
            key, expected_set2, actual_set2
        );
    }

    println!("   -> context_map is correct!\n\n\n");
}
fn test_enabled_log_activities(enabled_activities: &HashMap<String, HashSet<String>>, ocpn: OCPN) {
    test_enabled_activities(enabled_activities, true, ocpn);
}
fn test_enabled_model_activities(enabled_activities: &HashMap<String, HashSet<String>>, ocpn: OCPN) {
   test_enabled_activities(enabled_activities, false, ocpn);
}
fn test_enabled_activities(enabled_activities: &HashMap<String, HashSet<String>>, log: bool, ocpn: OCPN) {
    println!("#################### test enabled_activities ####################");

    let expected: HashMap<String, HashSet<String>> =
        if log {load_from_ron("src/test/Expected results (running example)/enabled_log_activities.ron").unwrap()}
        else {load_from_ron("src/test/Expected results (running example)/enabled_model_activities.ron").unwrap()};

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
            "Key '{}' not found in expected enabled_activities",
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

fn test_contexts(contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>){
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
                            event_id,
                            otype,
                            actual_vec
                        );
                        matched = true;
                        break;
                    }
                }

                assert!(
                    matched,
                    "Activity sequence {:?} (count {}) not found in expected for event '{}' and object type '{}'",
                    actual_vec,
                    actual_count,
                    event_id,
                    otype
                );
            }
        }
    }

    // All good
    println!("   -> Contexts are correct!\n\n\n");
}

fn test_bidnings(bindings: &HashMap<(String, String), HashMap<String, Vec<String>>>){

    //test
    println!("#################### test bindings ####################");

    //laod expected bindings of running example
    let expected: HashMap<(String, String), HashMap<String, Vec<String>>> = load_from_ron("src/test/Expected results (running example)/bindings.ron").unwrap();


    // compare the actual bindings with the expected ones

    assert_eq!(
        bindings.len(),
        expected.len(),
        "bindings.len (expected: {}, actual: {})",
        expected.len(),
        bindings.len()
    );

    //compare each entry
    for (key, actual_inner) in bindings {
        assert!(
            expected.contains_key(key),
            "Key {:?} not found",
            key
        );

        let expected_inner = &expected[key];

        // Vergleich der inneren HashMap
        assert_eq!(
            actual_inner.len(),
            expected_inner.len(),
            "inner map fpr {:?} has wrong length",
            key
        );

        for (inner_key, actual_vec) in actual_inner {
            assert!(
                expected_inner.contains_key(inner_key),
                "Innerer Key '{}' for {:?} not found",
                inner_key,
                key
            );

            // compare each vec
            let actual_set: HashSet<_> = actual_vec.iter().collect();
            let expected_set: HashSet<_> = expected_inner[inner_key].iter().collect();
            assert_eq!(
                actual_set, expected_set,
                "values for {:?}->{} are false (expected: {:?}, actual: {:?})",
                key, inner_key, expected_inner[inner_key], actual_vec
            );
        }
    }
    //everything is fine
    print!("   -> Bindings are correct!\n\n\n");
}









