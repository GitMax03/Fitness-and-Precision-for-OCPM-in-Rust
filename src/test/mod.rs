extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::fs::File;
use std::io::ErrorKind::StaleNetworkFileHandle;
use std::ops::Add;
use std::string::ToString;
use bimap::BiMap;
use pm::ocel::ocel_struct::*;
use chrono::{FixedOffset, TimeZone};
use petgraph::adj::NodeIndex;
use petgraph::graph::DiGraph;
use rustworkx_core::petgraph::dot::{Config, Dot};


use crate::{enabled_log_act, enabled_model_act};
use crate::enabled_log_act::{construct_event_object_graph, get_contexts_and_bindings, get_enabled_log_activities, get_event_presets};
use crate::structs_for_ocpm::*;


pub fn get_test_ocel() -> pm::OCEL {
    //get_test_ocel_sql()
    get_test_ocel_small()
}

pub fn test_enabled_model_act() {
    let ocel = get_test_ocel_small();
    let ocpn = get_test_ocpn_small();
    
    //get contexts and bindings
    let (contexts, bindings) = get_contexts_and_bindings(&ocel);
    
    let event_presets = get_event_presets(&ocel);
    
    //get enabled model activities
    //let enabled_model_activities = enabled_model_act::get_ebabled_model_activities_for_event(&"e1".to_string(), event_presets.get(&"e1".to_string()).unwrap(), &bindings, &ocpn);
    
    //println!("Enabled Model Activities: {:?}", enabled_model_activities);
}

pub fn test_ocpn() {
    let ocpn = get_test_ocpn_small();
    //visualize graph (export)
    ocpn.export_dot_to_file("src/test/vis.dot".to_string());
    
    let mut marking = Marking::new();
    marking.insert("pl2".to_string(), vec!["b1".to_string(), "b2".to_string()].iter().cloned().collect());
    marking.insert("pl1".to_string(), vec!["p1".to_string()].iter().cloned().collect());
    
    let mut binding_sequence: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    binding_sequence.insert("Check-in".to_string(), vec![vec!["b1".to_string()], vec!["b2".to_string()]]);
    
    

    assert_eq!(ocpn.remove_next_binding(&mut binding_sequence, &marking),
               Some(("Check-in".to_string(), vec!["b2".to_string()])));
    assert_eq!(ocpn.get_enabled_transitions_from_marking(&marking), 
               vec!["Check-in".to_string(), "Fuel plane".to_string()]);    
}





pub fn test_binding_sequence() {
    let ocel = get_test_ocel_small();
    
    let mut binding_sequences: HashMap<String, HashMap<String, Vec<Vec<String>>>> = HashMap::new();
    let mut presets = get_event_presets(&ocel);
    let (contexts, bindings) = get_contexts_and_bindings(&ocel);
    let mut used_objs : HashMap<String, HashSet<String>> = HashMap::new();
    for event in &ocel.events {
        let mut preset = presets.get_mut(&event.id).unwrap();
        preset.reverse();
        let (binding_sequence, used_obj) = enabled_model_act::get_binding_sequence_and_used_obj(&preset, &bindings.clone());
        binding_sequences.insert(event.id.clone(), binding_sequence);
        used_objs.insert(event.id.clone(), used_obj);
    }
    //print binding sequences
    println!("Binding Sequences:");
    println!("{:#?}", binding_sequences);
    
    println!("Used Objects:{:?}", used_objs);
    
    for sequence in binding_sequences {
        let event_id = sequence.0;
        let binding_sequence = sequence.1;
        println!("Event ID: {}", event_id);
        for (activity, object_ids) in binding_sequence {
            println!("  Activity: {}, Object IDs: {:?}", activity, object_ids);
        }
    }
}

pub fn test_enabled_log_activities() {
    let ocel = get_test_ocel();
    
    let (contexts, bindings) = get_contexts_and_bindings(&ocel);
    println!("Contexts: {:?}", contexts);
    //get enabled log activities
    let ela = get_enabled_log_activities(&ocel, &contexts);
    
    println!("Enabled Log Activities: {:?}", ela);
    
}

pub fn test_event_presets() {
    let ocel = get_test_ocel();
    
    //get event presets
    let presets = get_event_presets(&ocel);
    
    print_presets(&presets);
}
pub fn test_eog() {
    let ocel = get_test_ocel_sql();

    println!("start test");

    //println!("OCEL: {:?}", ocel);

    let eog = construct_event_object_graph(&ocel);
    
    //visualize graph (export)
    let dot = Dot::with_config(&eog, &[Config::EdgeNoLabel]);
    println!("{:?}", dot);

    //save eog to vis.fot
    let mut file = File::create("src/test/vis.dot").expect("error");
    write!(file, "{:?}", dot).expect("error");

    println!("graph saved in {:?}; run with: dot -Tpng vis.dot -o vis.png", file);
    
    
}

pub fn test_context_and_bindings() {

    //print_presets(&get_event_presets(&get_test_ocel()));
    
    let (ctxt, bind) = get_contexts_and_bindings(&get_test_ocel());
    
    
    
    println!("Contexts: {:?}", ctxt);
    print!("--------------------\n");
    println!("Bindings: {:?}", bind);
}


pub fn print_presets(presets: &HashMap<String, Vec<OCELEvent>>) {
    
    println!("Presets:");
    
    
    
    
    for preset in presets {
        let mut res_str = format!("{}: ", preset.0);
        res_str.push_str(&preset.1.iter()
            .map(|e| format!("{} ({})", e.id, e.event_type))
            .collect::<Vec<String>>()
            .join(", "));
        println!("{}", res_str);
    }
}



pub fn get_test_ocel_sql() -> pm::OCEL{
    pm::import_ocel_sqlite_from_path("/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/logistics.sqlite")
        .expect("Error importing OCEL from SQLite database")
}




// Create a small test OCEL for demonstration purposes

pub fn get_test_ocpn_small() -> OCPN {
    let mut objects_to_types: HashMap<String, String> = HashMap::new();
    
    
    objects_to_types.extend(vec![
        ("p1".to_string(), "plane".to_string()),
        ("p2".to_string(), "plane".to_string()),
        ("b1".to_string(), "baggage".to_string()),
        ("b2".to_string(), "baggage".to_string()),
        ("b3".to_string(), "baggage".to_string()),
        ("b4".to_string(), "baggage".to_string()),
    ]);

    let mut ocpn = OCPN {
        places: Default::default(),
        transitions: Default::default(),
        silent_transtions: Default::default(),
        arcs: vec![],
        initial_marking: None,
        final_marking: None,
        object_to_type: objects_to_types,
    };
    ocpn.add_place("pl1".to_string(), "plane".to_string());
    ocpn.add_place("pl2".to_string(), "baggage".to_string());
    ocpn.add_place("pl3".to_string(), "plane".to_string());
    ocpn.add_place("pl4".to_string(), "baggage".to_string());
    ocpn.add_place("pl5".to_string(), "plane".to_string());
    ocpn.add_place("pl6".to_string(), "baggage".to_string());
    ocpn.add_place("pl7".to_string(), "plane".to_string());
    ocpn.add_place("pl8".to_string(), "baggage".to_string());
    ocpn.add_place("pl9".to_string(), "plane".to_string());
    ocpn.add_place("pl10".to_string(), "plane".to_string());
    ocpn.add_place("pl11".to_string(), "baggage".to_string());

    ocpn.add_silent_transition("tau".to_string());


    ocpn.add_arc("pl1".to_string(), "Fuel plane".to_string(), "plane".to_string());
    ocpn.add_arc("pl2".to_string(), "Check-in".to_string(), "baggage".to_string());
    ocpn.add_arc("Fuel plane".to_string(), "pl3".to_string(), "plane".to_string());
    ocpn.add_arc("Check-in".to_string(), "pl4".to_string(), "baggage".to_string());
    ocpn.add_arc("pl3".to_string(), "Load cargo".to_string(), "plane".to_string());
    ocpn.add_arc("pl4".to_string(), "Load cargo".to_string(), "baggage".to_string());
    ocpn.add_arc("pl4".to_string(), "Load cargo".to_string(), "baggage".to_string());
    ocpn.add_arc("Load cargo".to_string(), "pl5".to_string(), "plane".to_string());
    ocpn.add_arc("Load cargo".to_string(), "pl6".to_string(), "baggage".to_string());
    ocpn.add_arc("Load cargo".to_string(), "pl6".to_string(), "baggage".to_string());
    ocpn.add_arc("pl5".to_string(), "Lift off".to_string(), "plane".to_string());
    ocpn.add_arc("pl6".to_string(), "Unload".to_string(), "baggage".to_string());
    ocpn.add_arc("pl6".to_string(), "Unload".to_string(), "baggage".to_string());
    ocpn.add_arc("pl6".to_string(), "tau".to_string(), "baggage".to_string());
    ocpn.add_arc("tau".to_string(), "pl8".to_string(), "baggage".to_string());
    ocpn.add_arc("Lift off".to_string(), "pl7".to_string(), "plane".to_string());
    ocpn.add_arc("pl7".to_string(), "Unload".to_string(), "plane".to_string());
    ocpn.add_arc("Unload".to_string(), "pl8".to_string(), "baggage".to_string());
    ocpn.add_arc("Unload".to_string(), "pl8".to_string(), "baggage".to_string());
    ocpn.add_arc("Unload".to_string(), "pl9".to_string(), "plane".to_string());
    ocpn.add_arc("pl8".to_string(), "Pick up @ dest".to_string(), "baggage".to_string());
    ocpn.add_arc("pl9".to_string(), "Clean".to_string(), "plane".to_string());
    ocpn.add_arc("Clean".to_string(), "pl10".to_string(), "plane".to_string());
    ocpn.add_arc("Pick up @ dest".to_string(), "pl11".to_string(), "baggage".to_string());


    let marking: Marking = HashMap::from([
        ("pl1".to_string(), HashSet::from(["p1".to_string()])),
        ("pl2".to_string(), HashSet::from(["b1".to_string(), "b2".to_string()])),
    ]);
    
    
    ocpn.initial_marking = Some(marking);
    
    ocpn
}


pub fn get_test_ocel_small() -> pm::OCEL {
    //create test ocel

    let empty_attribute = OCELTypeAttribute {
        name: "".to_string(),
        value_type: "".to_string(),
    };

    let event_types:Vec<OCELType> = vec![
        OCELType {
            name: "Fuel plane".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Check-in".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Load cargo".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Lift off".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Pick up @ dest".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Clean".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "Unload".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
    ];

    let object_types:Vec<OCELType> = vec![
        OCELType {
            name: "plane".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "baggage".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
    ];

    let events:Vec<OCELEvent> = create_events();


    let mut ocel = pm::OCEL {

        event_types: event_types,
        object_types: object_types,
        events: events,
        objects: vec![
            OCELObject {
                id: "p1".to_string(),
                object_type: "plane".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
                ],
            },
            OCELObject {
                id: "p2".to_string(),
                object_type: "plane".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
                ],
            },
            OCELObject {
                id: "b1".to_string(),
                object_type: "baggage".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
                ],
            },
            OCELObject {
                id: "b2".to_string(),
                object_type: "baggage".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
                ],
            },
            OCELObject {
                id: "b3".to_string(),
                object_type: "baggage".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
                ],
            },
            OCELObject {
                id: "b4".to_string(),
                object_type: "baggage".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
                ],
            },
        ],


    };
    
    ocel
}





fn create_events() -> Vec<OCELEvent> {
    let offset = FixedOffset::east_opt(3600).unwrap(); // UTC+1
    let mut time = offset.ymd(2023, 1, 1).and_hms(8, 0, 0); // Startzeit 08:00
    let empty_event_attribute = OCELEventAttribute {
        name: "".to_string(),
        value: OCELAttributeValue::Boolean(false),
    };

    vec![
        OCELEvent {
            id: "e1".to_string(),
            event_type: "Fuel plane".to_string(),
            time: time,
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
            ],
        },
        OCELEvent {
            id: "e2".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(1)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
            ],
        },
        OCELEvent {
            id: "e3".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(2)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "e4".to_string(),
            event_type: "Load cargo".to_string(),
            time: time.add(chrono::Duration::hours(3)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "e5".to_string(),
            event_type: "Lift off".to_string(),
            time: time.add(chrono::Duration::hours(4)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
            ],
        },
        OCELEvent {
            id: "e6".to_string(),
            event_type: "Unload".to_string(),
            time: time.add(chrono::Duration::hours(5)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "e7".to_string(),
            event_type: "Pick up @ dest".to_string(),
            time: time.add(chrono::Duration::hours(6)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
            ],
        },
        OCELEvent {
            id: "e8".to_string(),
            event_type: "Pick up @ dest".to_string(),
            time: time.add(chrono::Duration::hours(7)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "e9".to_string(),
            event_type: "Clean".to_string(),
            time: time.add(chrono::Duration::hours(8)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p1".to_string() },
            ],
        },
        OCELEvent {
            id: "e10".to_string(),
            event_type: "Fuel plane".to_string(),
            time: time.add(chrono::Duration::hours(9)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "e11".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(10)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
            ],
        },
        OCELEvent {
            id: "e12".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(11)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "e13".to_string(),
            event_type: "Load cargo".to_string(),
            time: time.add(chrono::Duration::hours(12)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "e14".to_string(),
            event_type: "Lift off".to_string(),
            time: time.add(chrono::Duration::hours(13)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "e15".to_string(),
            event_type: "Unload".to_string(),
            time: time.add(chrono::Duration::hours(14)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "e16".to_string(),
            event_type: "Clean".to_string(),
            time: time.add(chrono::Duration::hours(15)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "plane".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "e17".to_string(),
            event_type: "Pick up @ dest".to_string(),
            time: time.add(chrono::Duration::hours(16)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
            ],
        },
        OCELEvent {
            id: "e18".to_string(),
            event_type: "Pick up @ dest".to_string(),
            time: time.add(chrono::Duration::hours(17)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
    ]
}

