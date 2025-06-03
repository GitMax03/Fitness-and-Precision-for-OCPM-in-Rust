extern crate process_mining as pm;

use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::ops::Add;
use pm::ocel::ocel_struct::*;
use chrono::{FixedOffset, TimeZone};
use rustworkx_core::petgraph::dot::{Config, Dot};


use crate::context;
use crate::context::{construct_event_object_graph, get_contexts_and_bindings, get_event_presets};

pub fn test_eog() {
    let ocel = get_test_ocel();

    println!("start test");

    //println!("OCEL: {:?}", ocel);

    let eog = construct_event_object_graph(&ocel);
    
    //visualize graph (export)
    let dot = Dot::with_config(&eog, &[Config::EdgeNoLabel]);
    println!("{:?}", dot);

    //save eog to vis.fot
    let mut file = File::create("src/test/vis.dot").expect("error");
    write!(file, "{:?}", dot).expect("error");

    println!("graph saved in {:?}; run with dot -Tpng graph.dot -o graph.png", file);
    
    
}

pub fn test_context_and_bindings() {

    print_presets(&get_event_presets(&get_test_ocel()));
    
    let (ctxt, bind) = get_contexts_and_bindings(&get_test_ocel());
    
    
    
    println!("Contexts: {:?}", ctxt);
    print!("--------------------\n");
    println!("Bindings: {:?}", bind);
}


fn print_presets(presets: &HashMap<String, Vec<OCELEvent>>) {
    for preset in presets {
        println!("Preset: {:?}", preset);
    }
}


fn get_test_ocel() -> pm::OCEL {
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
            name: "clean".to_string(),
            attributes: vec![
                empty_attribute.clone(),
            ],
        },
        OCELType {
            name: "unload".to_string(),
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
            event_type: "unload".to_string(),
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
            event_type: "clean".to_string(),
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

