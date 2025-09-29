

extern crate process_mining as pm;

use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::fs::File;
use std::io::ErrorKind::StaleNetworkFileHandle;
use std::ops::Add;
use std::process::id;
use std::string::ToString;
use bimap::BiMap;
use pm::ocel::ocel_struct::*;
use chrono::{FixedOffset, TimeZone};
use itertools::assert_equal;
use petgraph::adj::NodeIndex;
use petgraph::graph::DiGraph;
use petgraph::dot::{Dot, Config};
use rustc_hash::FxHashSet;
use crate::{enabled_log_act, enabled_model_act};
use crate::enabled_log_act::{construct_event_object_graph, _get_contexts_and_bindings, get_enabled_log_activities, get_event_presets};
use crate::MDFA::{abstract_marking, abstract_marking_with_map, compare_marking_multiplicity, get_all_rename_maps, MDFA};
use crate::structs_for_ocpm::*;


pub fn get_test_ocel() -> pm::OCEL {
    //get_test_ocel_sql()
    get_test_ocel_small()
}



pub(crate) fn test_MDFA() {
    
    println!("MDFA");

    let object_to_type = HashMap::from([
        ("a1".to_string(), "a".to_string()),
        ("a2".to_string(), "a".to_string()),
        ("b1".to_string(), "b".to_string()),
        ("b2".to_string(), "b".to_string()),
        ("c1".to_string(), "c".to_string()),
        ("c2".to_string(), "c".to_string()),
    ]);
    
    let m1 = abstract_marking(&HashMap::from([
        ("p1".to_string(), HashSet::from(["a1".to_string()])),
        ("p2".to_string(), HashSet::from(["b1".to_string()])),
    ]), &object_to_type);
    let m1_ = HashMap::from([
        ("p5".to_string(), HashSet::from(["a1".to_string()])),
        ("p2".to_string(), HashSet::from(["b1".to_string()])),
    ]);
    let m1__ = HashMap::from([
        ("p5".to_string(), HashSet::from(["a1".to_string()])),
        ("p4".to_string(), HashSet::from(["b1".to_string()])),
    ]);
    
    let m2 = abstract_marking(&HashMap::from([
        ("p1".to_string(), HashSet::from(["a1".to_string()])),
        ("p2".to_string(), HashSet::from(["b1".to_string()])),
        ("px".to_string(), HashSet::from(["c1".to_string()])),
    ]), &object_to_type);
    let m2_ = HashMap::from([
        ("p5".to_string(), HashSet::from(["a1".to_string()])),
        ("p2".to_string(), HashSet::from(["b1".to_string()])),
        ("px".to_string(), HashSet::from(["c1".to_string()])),
    ]);
    let m2__ = HashMap::from([
        ("p5".to_string(), HashSet::from(["a1".to_string()])),
        ("p4".to_string(), HashSet::from(["b1".to_string()])),
        ("px".to_string(), HashSet::from(["c1".to_string()])),
    ]);
    let m2___ = HashMap::from([
        ("p3".to_string(), HashSet::from(["a1".to_string()])),
        ("p4".to_string(), HashSet::from(["b1".to_string()])),
        ("px".to_string(), HashSet::from(["c1".to_string()])),
    ]);
    



    
    
    //create mdfa
    let mut mdfa = MDFA::new(object_to_type.clone());
    mdfa.add_state(Some("1".to_string()), true);
    mdfa.states.get_mut("1").unwrap().write().unwrap().markings.extend(
        vec![
            (hash_marking(&m1, &mdfa.object_to_type, &None), vec![m1.clone()]),
            (hash_marking(&m2, &mdfa.object_to_type, &None), vec![m2.clone()]),
        ]
    );
    mdfa.add_state(Some("2".to_string()), false);
    mdfa.states.get_mut("2").unwrap().write().unwrap().markings.extend(
        vec![
            (hash_marking(&m1, &mdfa.object_to_type, &None), vec![abstract_marking(&mut m1_.clone(), &object_to_type), abstract_marking(&mut m1__.clone(), &object_to_type)]),
            (hash_marking(&m2, &mdfa.object_to_type, &None), vec![abstract_marking(&mut m2_.clone(), &object_to_type), abstract_marking(&mut m2__.clone(), &object_to_type)]),
        ]
    );
    mdfa.add_state(Some("3".to_string()), false);
    mdfa.states.get_mut("3").unwrap().write().unwrap().markings.extend(
        vec![
            (hash_marking(&m2, &mdfa.object_to_type, &None), vec![abstract_marking(&mut m2___.clone(), &object_to_type)]),
        ]
    );
    mdfa.add_arc(Some("1".to_string()), "A".to_string(), Some("2".to_string()));
    mdfa.add_arc(Some("1".to_string()), "B".to_string(), Some("3".to_string()));
    
    
    
    //example binding seq and Marking
    let t_marking = HashMap::from([
        ("p1".to_string(), HashSet::from(["a2".to_string()])),
        ("p2".to_string(), HashSet::from(["b2".to_string()])),
    ]);
    let t_binding_seq:Vec<(String, HashMap<String, HashSet<String>>)> = vec![
        ("C".to_string(), HashMap::from([("a".to_string(), HashSet::from(["a2".to_string()]))])),
        //("B".to_string(), HashMap::from([("b".to_string(), HashSet::from(["b2".to_string()]))])),
    ];
    
    //enabled Activitiy:
    let act:FxHashSet<String> = FxHashSet::from(["activity:1".to_string()].into_iter().collect());
    let temp = mdfa.states.get("2").unwrap().read().unwrap().markings.get(&hash_markings(&vec![m1.clone()], &mdfa.object_to_type)).unwrap().clone();
    mdfa.states.get_mut("2").unwrap().write().unwrap().enabled_activities.insert(hash_markings(
        &temp,
        &mdfa.object_to_type), act.clone());   
    print!("s");
    let initial_marking = t_marking.clone();
    let binding = t_binding_seq.clone();
    
    //OCPN
    
    
    
    println!("{:#?}", mdfa.states);
    
    //replay on MDFA
    for initial_state_mdfa in mdfa.initial_states.read().unwrap().iter() {
        let initial_state = mdfa.states.get(initial_state_mdfa).unwrap();


        //check if there is a fitting marking in this initial state
        for marking_init_ in initial_state.read().unwrap().markings.values() {

            //TODO: ASSUMPTION: initial markings are always Vecs with only one element!!!
            let marking_init = marking_init_.get(0).unwrap();

            if !compare_marking_multiplicity(marking_init, &initial_marking, &object_to_type.clone()) {
                //=> Marking can't be abstracted to initial marking
                continue;
            }

            //get all rename maps
            let rename_maps = get_all_rename_maps(&initial_marking, &object_to_type.clone());
            for map in rename_maps {
                //try if this one fits
                let abst_marking = abstract_marking_with_map(&initial_marking, &map);

                //check if marking is equal to initial marking of the mdfa
                if marking_init != &abst_marking { continue; }
                
                //try to replay binding on mdfa
                let (state, binding_seq, markings) = mdfa.replay_next_state(
                    initial_state.read().unwrap().clone(),
                    &vec![marking_init.clone()],
                    &binding,
                );
                
                println!("State: {:?}", state);
                println!("Binding seq: {:?}", binding_seq);
                println!("H_marking: {:?}, marking: {:?}", hash_markings(&markings, &mdfa.object_to_type), markings);
                let x = mdfa.states.get("2").unwrap().read().unwrap().markings.get(&hash_markings(&vec![m1.clone()], &mdfa.object_to_type)).cloned().unwrap();
                println!("State: {:?}; key:{:?}", x.clone(), hash_markings(&vec![m1.clone()], &mdfa.object_to_type));
                println!("{:?}", x == markings);
                println!("Enabled Activities: {:?}", state.enabled_activities.get(&hash_markings(&markings, &mdfa.object_to_type)).unwrap_or(&FxHashSet::default()));

            }
        }
    }

    println!("hash marking1: {:?};; hashmakrings: {:?}", hash_markings(&vec![m1_, m1__], &mdfa.object_to_type), hash_markings(&vec![m1], &mdfa.object_to_type));
}


 fn test_event_presets() {
    let ocel = get_test_ocel();
    let id_to_event: HashMap<String, OCELEvent> = ocel.events.iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    //get event presets
    let presets = get_event_presets(&ocel, &id_to_event);

    print_presets(&presets.0);
}
pub fn test_eog() {
    let ocel = get_test_ocel();

    println!("start test");

    //println!("OCEL: {:?}", ocel);

    let (eog,_) = construct_event_object_graph(&ocel);
/*
    //visualize graph (export)
    let dot = Dot::with_config(&eog, &[Config::EdgeNoLabel]);
    println!("{:?}", dot);

    //save eog to vis.fot
    let mut file = File::create("src/test/vis.dot").expect("error");
    write!(file, "{:?}", dot).expect("error");

    println!("graph saved in {:?}; run with: dot -Tpng vis.dot -o vis.png", file);
*/

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
        ("p:1".to_string(), "pla:ne".to_string()),
        ("p2".to_string(), "pla:ne".to_string()),
        ("b1".to_string(), "baggage".to_string()),
        ("b2".to_string(), "baggage".to_string()),
        ("b3".to_string(), "baggage".to_string()),
        ("b4".to_string(), "baggage".to_string()),
    ]);

    let mut ocpn = OCPN::new(objects_to_types);

    ocpn.add_place(Some("pl1".to_string()), "pla:ne".to_string(), true);
    ocpn.add_place(Some("pl2".to_string()), "baggage".to_string(), true);
    ocpn.add_place(Some("pl3".to_string()), "pla:ne".to_string(), false);
    ocpn.add_place(Some("pl4".to_string()), "baggage".to_string(), false);
    ocpn.add_place(Some("pl5".to_string()), "pla:ne".to_string(), false);
    ocpn.add_place(Some("pl6".to_string()), "baggage".to_string(), false);
    ocpn.add_place(Some("pl7".to_string()), "pla:ne".to_string(), false);
    ocpn.add_place(Some("pl8".to_string()), "baggage".to_string(), false);
    ocpn.add_place(Some("pl9".to_string()), "pla:ne".to_string(), false);
    ocpn.add_place(Some("pl10".to_string()), "pla:ne".to_string(), false);
    ocpn.add_place(Some("pl11".to_string()), "baggage".to_string(), false);

    ocpn.add_silent_transition("tau".to_string(), Some("tau".to_string()));


    ocpn.add_arc(Some("pl1".to_string()),           Some("Fuel plane".to_string()),     "pla:ne".to_string(),    Some("Fuel plane".to_string()));
    ocpn.add_arc(Some("pl2".to_string()),           Some("Check-in".to_string()),       "baggage".to_string(),  Some("Check-in".to_string()));
    ocpn.add_arc(Some("Fuel plane".to_string()),    Some("pl3".to_string()),            "pla:ne".to_string(),    Some("Fuel plane".to_string()));
    ocpn.add_arc(Some("Check-in".to_string()),      Some("pl4".to_string()),            "baggage".to_string(),  Some("Check-in".to_string()));
    ocpn.add_arc(Some("pl3".to_string()),           Some("Load cargo".to_string()),     "pla:ne".to_string(),    Some("Load cargo".to_string()));
    ocpn.add_arc(Some("pl4".to_string()),           Some("Load cargo".to_string()),     "baggage".to_string(),  Some("Load cargo".to_string()));
    ocpn.add_arc(Some("pl4".to_string()),           Some("Load cargo".to_string()),     "baggage".to_string(),  Some("Load cargo".to_string()));
    ocpn.add_arc(Some("Load cargo".to_string()),    Some("pl5".to_string()),            "pla:ne".to_string(),    Some("Load cargo".to_string()));
    ocpn.add_arc(Some("Load cargo".to_string()),    Some("pl6".to_string()),            "baggage".to_string(),  Some("Load cargo".to_string()));
    ocpn.add_arc(Some("Load cargo".to_string()),    Some("pl6".to_string()),            "baggage".to_string(),  Some("Load cargo".to_string()));
    ocpn.add_arc(Some("pl6".to_string()),           Some("Unload".to_string()),         "baggage".to_string(),  Some("Unload".to_string()));
    ocpn.add_arc(Some("pl6".to_string()),           Some("Unload".to_string()),         "baggage".to_string(),  Some("Unload".to_string()));
    ocpn.add_arc(Some("pl5".to_string()),           Some("Lift off".to_string()),       "pla:ne".to_string(),    Some("Lift off".to_string()));
    ocpn.add_arc(Some("pl6".to_string()),           Some("tau".to_string()),            "baggage".to_string(),  Some("tau".to_string()));
    ocpn.add_arc(Some("tau".to_string()),           Some("pl8".to_string()),            "baggage".to_string(),  Some("tau".to_string()));
    ocpn.add_arc(Some("Lift off".to_string()),      Some("pl7".to_string()),            "pla:ne".to_string(),    Some("Lift off".to_string()));
    ocpn.add_arc(Some("pl7".to_string()),           Some("Unload".to_string()),         "pla:ne".to_string(),    Some("Unload".to_string()));
    ocpn.add_arc(Some("Unload".to_string()),        Some("pl8".to_string()),            "baggage".to_string(),  Some("Unload".to_string()));
    ocpn.add_arc(Some("Unload".to_string()),        Some("pl8".to_string()),            "baggage".to_string(),  Some("Unload".to_string()));
    ocpn.add_arc(Some("Unload".to_string()),        Some("pl9".to_string()),            "pla:ne".to_string(),    Some("Unload".to_string()));
    ocpn.add_arc(Some("pl8".to_string()),           Some("Pick up @ dest".to_string()), "baggage".to_string(),  Some("Pick up @ dest".to_string()));
    ocpn.add_arc(Some("pl9".to_string()),           Some("Clean".to_string()),          "pla:ne".to_string(),    Some("Clean".to_string()));
    ocpn.add_arc(Some("Clean".to_string()),         Some("pl10".to_string()),           "pla:ne".to_string(),    Some("Clean".to_string()));
    ocpn.add_arc(Some("Pick up @ dest".to_string()),Some("pl11".to_string()),           "baggage".to_string(),  Some("Pick up @ dest".to_string()));


    let marking: Marking = HashMap::from([
        ("pl1".to_string(), HashSet::from(["p:1".to_string()])),
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
            name: "pla:ne".to_string(),
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
                id: "p:1".to_string(),
                object_type: "pla:ne".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
                ],
            },
            OCELObject {
                id: "p2".to_string(),
                object_type: "pla:ne".to_string(),
                attributes: vec![
                ],
                relationships: vec![
                    OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
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
            id: "1".to_string(),
            event_type: "Fuel plane".to_string(),
            time: time,
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
            ],
        },
        OCELEvent {
            id: "2".to_string(),
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
            id: "3".to_string(),
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
            id: "4".to_string(),
            event_type: "Load cargo".to_string(),
            time: time.add(chrono::Duration::hours(3)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "5".to_string(),
            event_type: "Lift off".to_string(),
            time: time.add(chrono::Duration::hours(4)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
            ],
        },
        OCELEvent {
            id: "6".to_string(),
            event_type: "Unload".to_string(),
            time: time.add(chrono::Duration::hours(5)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b1".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b2".to_string() },
            ],
        },
        OCELEvent {
            id: "7".to_string(),
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
            id: "8".to_string(),
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
            id: "9".to_string(),
            event_type: "Clean".to_string(),
            time: time.add(chrono::Duration::hours(8)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p:1".to_string() },
            ],
        },
        OCELEvent {
            id: "10".to_string(),
            event_type: "Fuel plane".to_string(),
            time: time.add(chrono::Duration::hours(9)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "11".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(10)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
            ],
        },
        OCELEvent {
            id: "12".to_string(),
            event_type: "Check-in".to_string(),
            time: time.add(chrono::Duration::hours(11)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "13".to_string(),
            event_type: "Load cargo".to_string(),
            time: time.add(chrono::Duration::hours(12)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "14".to_string(),
            event_type: "Lift off".to_string(),
            time: time.add(chrono::Duration::hours(13)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "15".to_string(),
            event_type: "Unload".to_string(),
            time: time.add(chrono::Duration::hours(14)),
            attributes: vec![],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b3".to_string() },
                OCELRelationship { qualifier: "baggage".to_string(), object_id: "b4".to_string() },
            ],
        },
        OCELEvent {
            id: "16".to_string(),
            event_type: "Clean".to_string(),
            time: time.add(chrono::Duration::hours(15)),
            attributes: vec![
                empty_event_attribute.clone(),
            ],
            relationships: vec![
                OCELRelationship { qualifier: "pla:ne".to_string(), object_id: "p2".to_string() },
            ],
        },
        OCELEvent {
            id: "17".to_string(),
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
            id: "18".to_string(),
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

