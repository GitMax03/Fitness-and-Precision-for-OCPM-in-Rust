use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::format;
use std::fs::{OpenOptions, read_to_string};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::iter::zip;
use chrono::{Local, TimeZone};
use std::process::{id, Command};
use std::time::Instant;
use counter::Counter;
use process_mining::{import_ocel_xml_file, OCEL};
use multiset::HashMultiSet;
use multiset_hash::RistrettoHash;
use process_mining::ocel::ocel_struct::{OCELAttributeValue, OCELEvent, OCELEventAttribute, OCELObject, OCELObjectAttribute, OCELRelationship, OCELType, OCELTypeAttribute};
use crate::{enabled_log_act, utils, NUMBER_OF_ITERATIONS, SAVE};
use crate::structs_for_ocpm::{Marking, OCPN};


pub fn evaluate_runtime(starting_times:&Vec<Instant>, steps_as_strings:&Vec<String>, endtime:Instant) -> (Vec<(String, f64)>, Vec<(String, f32)>) {

    
    println!("\n\n###############################################################");
    println!("evaluating runtime ...");
    println!("###############################################################\n\n");
    
    if starting_times.len() != steps_as_strings.len() + 1 {
        println!("Steps length mismatch.");
        
        println!("Starting times:{:?}", starting_times);
        println!("Steps:{:?}", steps_as_strings);
        
    }
    
    let mut task_time_map:Vec<(String,f64)> = Vec::new();

    for index in (1..starting_times.len()) {
        
        //calc elapes time for this step
        let elapsed_time = (starting_times[index] - starting_times[index - 1]) / NUMBER_OF_ITERATIONS.into();
        
        task_time_map.push((steps_as_strings[index - 1].to_string(), elapsed_time.as_secs_f64()));
        
        println!("{:<50} took {} seconds", steps_as_strings[index - 1], elapsed_time.as_secs_f64());
    }
    
    println!("----------------------------------------------------------------------------------------");


    
    task_time_map.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, time) in &task_time_map {
        println!("{:<50} : {} seconds", name, time);
    }

    println!("----------------------------------------------------------------------------------------");
    let total_time = endtime - starting_times.first().unwrap().clone();
    
    println!("Total time: {} Seconds With {} iterations", total_time.as_secs_f64(), NUMBER_OF_ITERATIONS);
    println!("Average time: {} Seconds", (total_time.as_secs_f64() / NUMBER_OF_ITERATIONS as f64));
    println!("----------------------------------------------------------------------------------------");
    
    let mut percentages: Vec<(String,f32)> = Vec::new();
    let sum = task_time_map.iter().map(|(a,b)|b).sum::<f64>();
    for (task, time) in task_time_map.clone(){
        percentages.push((task, (time / sum * 100.09) as f32));
    }
    percentages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, percentage) in &percentages.clone() {
        println!("{:<50} : {} %", name, percentage);
    }
    
    println!("\n\n #####################################################################################\n\n\n");

    (task_time_map, percentages)
}




pub fn save_analytics (name: String, model:String, starting_times:&Vec<Instant>, steps_as_strings:&Vec<String>) -> std::io::Result<()>{

    let old_content = read_to_string("src/test/analytics_log.txt").unwrap_or_default();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true) // Datei überschreiben
        .open("src/test/analytics_log.txt")?;

    let (task_time_map, percentages) = evaluate_runtime(starting_times, steps_as_strings, Instant::now());

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let title = format!("### Run {} time: {} log/model: {}\n\n", name, timestamp, model);
    let mut separator = format!("\n{}\n\n\n\n", "-".repeat(120));
    
    
    //format new entry
    let total_time = Instant::now() - starting_times.first().unwrap().clone();
    
    let mut analytics = "".to_string();
    analytics = analytics + "\n-------------Task Times-----------\n";
    for (name, time) in &task_time_map {
        analytics = analytics + (format!("{:<50} : {} seconds", name, time).as_str()) + "\n";
    }
    analytics = analytics + "\n-------------Percentages-----------\n";
    for (name, percentage) in &percentages.clone() {
        analytics = analytics + format!("{:<50} : {} %", name, percentage).as_str() + "\n";
    }
    analytics = analytics + "\n-------------Total Time-----------\n";
    analytics = analytics + format!("{} Seconds With {} iterations", total_time.as_secs_f64(), NUMBER_OF_ITERATIONS).as_str() + "\n";
    
    analytics = analytics + "\n-------------Average Time-----------\n";
    analytics = analytics + format!("Average time: {} Seconds", (total_time.as_secs_f64() / NUMBER_OF_ITERATIONS as f64)).as_str();
    
    analytics = analytics + "\n\n";

    //check if it is new version
    let last_header = old_content.clone()
        .lines()
        .find(|line| line.starts_with("###"))
        .map(|line| line.trim_start_matches("### ").to_string());
    
    let last_version = last_header.as_ref()
        .and_then(|header| header.strip_prefix("Run "))
        .and_then(|s| s.split_once(" time"))
        .map(|(version, _)| version);
    println!("\n{}", last_version.unwrap());
    
    if last_version.clone().unwrap() != name {
        separator = separator.clone() + &*separator + "\n\n\n\n";
    }

    // create new entry
    let new_entry = format!("{}{}{}", title, analytics, separator);

    // save new entry
    

    println!("Save analyics...");
    write!(file, "{}{}", new_entry, old_content)?;

    Ok(())
    
}

pub fn import_ocel_from_jsonocel(json_str: &str) -> OCEL {

    let empty_event_attribute = vec![OCELEventAttribute {
        name: "".to_string(),
        value: OCELAttributeValue::Boolean(false),
    }];
    let empty_type_attribute = vec![OCELTypeAttribute {
        name: "".to_string(),
        value_type: "".to_string(),
    }];
    
    //read OCEL from JSON file
    let json_value: serde_json::Value = serde_json::from_str(&*read_to_string(json_str).unwrap()).unwrap();
    
    
    
    let mut objects_to_type: HashMap<String, String> = HashMap::new();
    let mut object_types: Vec<OCELType> = Vec::new();
    let mut objects: Vec<OCELObject> = Vec::new();
    
    for (object_id, object_val) in json_value["ocel:objects"].as_object().unwrap() {
        let object_type = object_val["ocel:type"].as_str().unwrap_or("").to_string();


        //for ocel1.0 jsonocel
        //edit object_id
        let object_id = object_id.replace(":", "");//.replace(" ", "_");



        objects_to_type.insert(object_id.clone(), object_type.clone());
        
        object_types.push(OCELType {
            name: object_type.clone(),
            attributes: empty_type_attribute.clone(),
        });
        objects.push(OCELObject {
            id: object_id.clone(),
            object_type: object_type.clone(),
            attributes: vec![],
            relationships: vec![
                OCELRelationship {
                    qualifier: object_type.clone(),
                    object_id: object_id.clone(),
                }
            ],
        });
    }
    
    
    let mut activitys: HashSet<String> = HashSet::new();
    let mut events: Vec<OCELEvent> = Vec::new();

    //------
    let events_ = json_value["ocel:events"]
        .as_object()
        .ok_or("'ocel:events' not found").unwrap();


    for (index, (event_id, event_val)) in events_.iter().enumerate() {
        let mut event: OCELEvent;

        let mut relationships: Vec<OCELRelationship> = Vec::new();

        for object_id in event_val["ocel:omap"].as_array().unwrap() {

            if objects_to_type.get(object_id.as_str().unwrap()).is_none() {
                println!("{} is not an OCEL object", object_id);
            }

            relationships.append(vec![OCELRelationship{
                qualifier: objects_to_type.get(object_id.as_str().unwrap()).unwrap().to_string(),
                object_id: object_id.as_str().unwrap_or("").to_string(),
            }].as_mut());
        }


        let mut attributes: Vec<OCELEventAttribute> = Vec::new();
        if let Some(vmap) = event_val["ocel:vmap"].as_object() {
            for (attr_name, attr_val) in vmap.iter() {
                // Datentyp bestimmen (hier vereinfacht nur String, Zahl, Bool)
                let value = if let Some(b) = attr_val.as_bool() {
                    OCELAttributeValue::Boolean(b)
                } else if let Some(n) = attr_val.as_f64() {
                    OCELAttributeValue::Float(n)
                } else if let Some(s) = attr_val.as_str() {
                    OCELAttributeValue::String(s.to_string())
                } else {
                    OCELAttributeValue::String(format!("{:?}", attr_val)) // fallback
                };

                attributes.push(OCELEventAttribute {
                    name: attr_name.clone(),
                    value,
                });
            }
        }


        activitys.insert(event_val["ocel:activity"].as_str().unwrap_or("").to_string());

        event = OCELEvent {
            id: index.clone().to_string(),
            event_type: event_val["ocel:activity"].as_str().unwrap_or("").to_string(),
            time: chrono::DateTime::parse_from_rfc3339(event_val["ocel:timestamp"].as_str().unwrap_or("")).unwrap_or_else(|_| chrono::FixedOffset::east_opt(0).unwrap().timestamp(0, 0)),
            attributes,
            relationships,
        };
        events.push(event);
    }

    //-----

    let mut event_types: Vec<OCELType> = Vec::new();
    for activity in activitys {
        event_types.push(OCELType {
            name: activity.clone(),
            attributes: empty_type_attribute.clone(),
        });
    }
    
    
    OCEL {
        objects: objects,
        object_types: object_types,
        events: events,
        event_types: event_types,
    }
}

pub fn import_ocel2_from_jsonocel(json_str: &str) -> OCEL {

    let empty_event_attribute = vec![OCELEventAttribute {
        name: "".to_string(),
        value: OCELAttributeValue::Boolean(false),
    }];
    let empty_type_attribute = vec![OCELTypeAttribute {
        name: "".to_string(),
        value_type: "".to_string(),
    }];

    //read OCEL from JSON file
    let json_value: serde_json::Value = serde_json::from_str(&*read_to_string(json_str).unwrap()).unwrap();



    let mut objects_to_type: HashMap<String, String> = HashMap::new();
    let mut object_types: Vec<OCELType> = Vec::new();
    let mut objects: Vec<OCELObject> = Vec::new();

    for value in json_value["objects"].as_array().unwrap() {
        
        let object_type = value["type"].as_str().unwrap_or("").to_string();
        let object_id = value["id"].as_str().unwrap_or("").to_string();
        
        objects_to_type.insert(object_id.clone(), object_type.clone());

        object_types.push(OCELType {
            name: object_type.clone(),
            attributes: empty_type_attribute.clone(),
        });
        objects.push(OCELObject {
            id: object_id.clone(),
            object_type: object_type.clone(),
            attributes: vec![],
            relationships: vec![
                OCELRelationship {
                    qualifier: object_type.clone(),
                    object_id: object_id.clone(),
                }
            ],
        });
    }


    let mut activitys: HashSet<String> = HashSet::new();
    let mut events: Vec<OCELEvent> = Vec::new();

    //------
    let events_ = json_value["events"]
        .as_array()
        .ok_or("'ocel:events' not found").unwrap();


    for (index, value) in events_.iter().enumerate() {
        
        let event_val = value.as_object().unwrap();
        
        let mut event: OCELEvent;

        let mut relationships: Vec<OCELRelationship> = Vec::new();

        
        for rel in event_val["relationships"].as_array().unwrap() {
            let object_id = rel["objectId"].as_str().unwrap_or("").to_string();
            relationships.append(vec![OCELRelationship{
                qualifier: objects_to_type.get(object_id.as_str()).unwrap().to_string(),
                object_id: object_id.as_str().to_string(),
            }].as_mut());
        }

        activitys.insert(event_val["type"].as_str().unwrap_or("").to_string());

        event = OCELEvent {
            id: index.clone().to_string(),
            event_type: event_val["type"].as_str().unwrap_or("").to_string(),
            time: chrono::DateTime::parse_from_rfc3339(event_val["time"].as_str().unwrap_or("")).unwrap_or_else(|_| chrono::FixedOffset::east_opt(0).unwrap().timestamp(0, 0)),
            attributes: empty_event_attribute.clone(),
            relationships,
        };
        events.push(event);
    }

    //-----

    let mut event_types: Vec<OCELType> = Vec::new();
    for activity in activitys {
        event_types.push(OCELType {
            name: activity.clone(),
            attributes: empty_type_attribute.clone(),
        });
    }


    OCEL {
        objects: objects,
        object_types: object_types,
        events: events,
        event_types: event_types,
    }
}

pub fn import_ocel_from_json_file(json_str: &str) -> OCEL {

    let empty_event_attribute = vec![OCELEventAttribute {
        name: "".to_string(),
        value: OCELAttributeValue::Boolean(false),
    }];
    let empty_type_attribute = vec![OCELTypeAttribute {
        name: "".to_string(),
        value_type: "".to_string(),
    }];

    //read OCEL from JSON file
    let json_value: serde_json::Value = serde_json::from_str(&*read_to_string(json_str).unwrap()).unwrap();


    let mut objects_to_type: HashMap<String, String> = HashMap::new();
    let mut object_types: Vec<OCELType> = Vec::new();
    let mut objects: Vec<OCELObject> = Vec::new();
    let mut event_types: Vec<OCELType> = Vec::new();
    let mut events: Vec<OCELEvent> = Vec::new();

    if let Some(obj_types) = json_value["objectTypes"].as_array() {
        for ot in obj_types {
            let name = ot["name"].as_str().unwrap_or("").to_string();
            let mut attrs: Vec<OCELTypeAttribute> = Vec::new();
            if let Some(type_attrs) = ot["attributes"].as_array() {
                for a in type_attrs {
                    attrs.push(OCELTypeAttribute {
                        name: a["name"].as_str().unwrap_or("").to_string(),
                        value_type: a["type"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            object_types.push(OCELType { name, attributes: attrs });
        }
    }

    if let Some(ev_types) = json_value["eventTypes"].as_array() {
        for et in ev_types {
            let name = et["name"].as_str().unwrap_or("").to_string();
            let mut attrs: Vec<OCELTypeAttribute> = Vec::new();
            if let Some(type_attrs) = et["attributes"].as_array() {
                for a in type_attrs {
                    attrs.push(OCELTypeAttribute {
                        name: a["name"].as_str().unwrap_or("").to_string(),
                        value_type: a["type"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            event_types.push(OCELType { name, attributes: attrs });
        }
    }

    if let Some(objs) = json_value["objects"].as_array() {
        for o in objs {
            let id = o["id"].as_str().unwrap_or("").to_string();
            let object_type = o["type"].as_str().unwrap_or("").to_string();
            objects_to_type.insert(id.clone(), object_type.clone());


            // Relationships
            let mut rels: Vec<OCELRelationship> = Vec::new();
            if let Some(obj_rels) = o["relationships"].as_array() {
                for r in obj_rels {
                    rels.push(OCELRelationship {
                        object_id: r["objectId"].as_str().unwrap_or("").to_string(),
                        qualifier: r["qualifier"].as_str().unwrap_or("").to_string(),
                    });
                }
            }

            objects.push(OCELObject {
                id,
                object_type,
                attributes: vec![],
                relationships: rels,
            });
        }
    }


    if let Some(evs) = json_value["events"].as_array() {
        for (index, e) in evs.iter().enumerate() {
            let id = index.clone().to_string();
            let event_type = e["type"].as_str().unwrap_or("").to_string();
            let time = chrono::DateTime::parse_from_rfc3339(
                e["time"].as_str().unwrap_or("")
            ).unwrap_or_else(|_| chrono::FixedOffset::east_opt(0).unwrap().timestamp(0, 0));


            let mut rels: Vec<OCELRelationship> = Vec::new();
            if let Some(ev_rels) = e["relationships"].as_array() {
                for r in ev_rels {
                    rels.push(OCELRelationship {
                        object_id: r["objectId"].as_str().unwrap_or("").to_string(),
                        qualifier: r["qualifier"].as_str().unwrap_or("").to_string(),
                    });
                }
            }

            events.push(OCELEvent {
                id,
                event_type,
                time,
                attributes: vec![],
                relationships: rels,
            });
        }
    }



    OCEL {
        objects: objects,
        object_types: object_types,
        events: events,
        event_types: event_types,
    }
}


pub fn import_from_json(json_str: &str) -> OCPN {

    //import OCPN from JSON string
    let json_value: serde_json::Value = serde_json::from_str(&*read_to_string(json_str).unwrap()).unwrap();

    let object_to_type: HashMap<String, String> = json_value["object_to_type"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();



    let mut ocpn = OCPN::new(object_to_type); //create new OCPN

    for (place_id, place_val) in json_value["places"].as_object().unwrap() {
        let object_type = place_val["object_type"].as_str().unwrap();
        let is_initial = place_val["initial"].as_bool().unwrap();

        ocpn.add_place(Some(place_id.clone()), object_type.to_string(), is_initial);
    }

    for (transition_id, transition_val) in json_value["transitions"].as_object().unwrap() {
        let activity = transition_val["activity"].as_str().unwrap();
        let is_silent = false;

        ocpn.add_transition(activity.to_string(), Some(transition_id.clone()), Some(is_silent)).expect("Error adding transition");
    }
    for (silent_transition_id, silent_transition_val) in json_value["silent_transitions"].as_object().unwrap() {
        let activity = silent_transition_val["activity"].as_str().unwrap();
        let is_silent = true;

        ocpn.add_transition(activity.to_string(), Some(silent_transition_id.clone()), Some(is_silent)).expect("Error adding silent transition");
    }

    for (arc_id, arc_val) in json_value["arcs"].as_object().unwrap() {
        let source = arc_val["source"].as_str().unwrap();
        let target = arc_val["target"].as_str().unwrap();
        let object_type = arc_val["object_type"].as_str().unwrap();
        let activity = arc_val.get("activity").and_then(|v| v.as_str()).map(|s| s.to_string());

        ocpn.add_arc(Some(source.to_string()), Some(target.to_string()), object_type.to_string(), activity).expect("Error adding arc");
    }

    ocpn
}




/// Entfernt Leerzeichen aus allen relevanten Attributen in einer OCEL 2.0 JSON-Datei.
/// - input_file: Pfad zur Eingabedatei
/// - output_file: Pfad zur Ausgabedatei
pub fn clean_ocel_ids_and_types_for_json(input_file: &str, output_file: &str) {
    // JSON-Datei einlesen
    let data = fs::read_to_string(input_file).unwrap();
    let mut json: Value = serde_json::from_str(&data).unwrap();

    // Attribute, die geprüft werden sollen
    let target_attributes = ["id", "object-id", "object-type", "type", "qualifier", "name", "type", "objectId",
    "ocel:type", "ocel:activity", "ocel:omap", "", "ocel:objects"];

    // Rekursive Hilfsfunktion
    fn clean_json(value: &mut Value, targets: &[&str]) {
        match value {
            Value::Object(map) => {
                for (k, v) in map.iter_mut() {
                    if targets.contains(&k.as_str()) {
                        if let Value::String(s) = v {
                            let mut cleaned = s.replace(' ', "_");
                            //added for ocel 1.0 jsnonocel
                            cleaned = s.replace(":", "");
                            if *s != cleaned {
                                println!("Fix {}: '{}' -> '{}'", k, s, cleaned);
                                *s = cleaned;
                            }
                        }
                    }






                    // Rekursion auf alle Kindwerte
                    clean_json(v, targets);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {

                    //for ocel1.0 jsonocel
                    //check if v is string
                    if let Value::String(s) = v {
                        let mut cleaned = s.replace(' ', "_");
                        cleaned = s.replace(":", "");
                        if *s != cleaned {
                            println!("Fix string: '{}' -> '{}'", s, cleaned);
                            *s = cleaned;
                        }
                    }

                    clean_json(v, targets);
                }
            }
            _ => {}
        }
    }

    // get new event ids
    if let Some(events) = json["events"].as_array_mut() {
        for (i, ev) in events.iter_mut().enumerate() {
            ev["id"] = Value::String(i.to_string());
        }
    }

    // Reinigung anwenden
    clean_json(&mut json, &target_attributes);

    // Neue Datei schreiben
    fs::write(output_file, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}


pub fn clean_ocel(mut ocel:OCEL) -> OCEL{

    //rename events
    for (index, event) in ocel.events.iter_mut().enumerate(){
        event.id = index.to_string();
    }


    for obj in ocel.objects.iter_mut(){
        obj.id = obj.id.replace(" ", "_");
        obj.object_type = obj.object_type.replace(" ", "_");
        for rel in obj.relationships.iter_mut(){
            rel.object_id = rel.object_id.replace(" ", "_");
            rel.qualifier = rel.qualifier.replace(" ", "_");
        }
    }
    for ev in ocel.events.iter_mut(){

        for rel in ev.relationships.iter_mut(){
            rel.object_id = rel.object_id.replace(" ", "_");
            rel.qualifier = rel.qualifier.replace(" ", "_");
        }
        ev.event_type = ev.event_type.replace(" ", "_");
    }
    for ot in ocel.object_types.iter_mut(){
        ot.name = ot.name.replace(" ", "_");
    }
    for et in ocel.event_types.iter_mut(){
        et.name = et.name.replace(" ", "_");
    }
    ocel
}














//TODO provably already exists
pub fn has_intersection(a: &[String], b: &[String]) -> bool {
    // no _is_disjoint func for vec?
    let set_a: HashSet<_> = a.iter().collect();  
    b.iter().any(|s| set_a.contains(s))         
}



//check if a is superset of b (i.e. a contains all elements of b)
//TODO exisits already
pub fn is_superset(superset: &Vec<String>, subset: &Vec<String>) -> bool {
    let superset_set: HashSet<_> = superset.iter().collect();
    subset.iter().all(|item| superset_set.contains(item))
}

//TODO: mybe change vec<String> to HashSet<String>? for better efficiency
pub fn pop_object_from_binding(binding: &mut (String, Vec<String>), object_type: &String, object_to_type: &HashMap<String, String>) -> Option<String> {

    // get position of first object that matches the object_type
    let pos = binding.1.iter().position(|x| object_to_type.get(x).unwrap() == object_type);

    // pop the object at that position
    pos.map(|p| binding.1.remove(p))
    
}



//-----pretty print functions for debug-----------
pub fn print_contexts(contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>) {
    
    //sort contexts by event_id
    let sorted_contexts = sort_hashmap_by_numeric_keys::<String, HashMap<String, Counter<Vec<String>>>>(&contexts);
    
    
    
    println!("Contexts:");
    for (event_type, context_map) in sorted_contexts {
        println!("Event Type: {}", event_type);
        for (sorted_contexts, bindings) in context_map {
            println!("  Context: {}", sorted_contexts);
            for (binding, count) in bindings.iter() {
                println!("    Activities: {:?}, Count: {}", binding, count);
            }
        }
    }
    println!("____________________________________________");
}


pub fn sort_hashmap_by_numeric_keys<K, V>(
    data: &HashMap<String, V>,
) -> Vec<(&String, &V)> {
    let mut entries: Vec<_> = data.iter().collect();

    entries.sort_by_key(|(key, _)| {
        key.chars()
            .filter_map(|c| c.to_digit(10))  // Extrahiere einzelne Ziffern
            .fold(0, |acc, digit| acc * 10 + digit)  // Kombiniere zu einer Zahl
    });

    entries
}
///comare 2 bindings
pub fn equals(a: &Vec<(String, HashMap<String, HashSet<String>>)>, b: &Vec<(String, HashMap<String, HashSet<String>>)>) -> bool{
    //outer Vec is for equal bindings in same order
    if a.len() != b.len() {
        return false;
    }
    for ((a_act, a_binding), (b_act, b_binding)) in a.iter().zip(b.iter()){
        if a_act.len() != b_act.len() { return false; }
        
        if a_act != b_act { return false; }
    }
    return true;
}

pub fn ocel_preprocessing(ocel:OCEL) -> OCEL{
    let mut res: OCEL = ocel.clone();

    let replace = "";

    //remove all ":" from objects and object types and event relations
    for obj in res.objects.iter_mut(){
        obj.id = obj.id.replace(":", replace);
        obj.object_type = obj.object_type.replace(":", replace);
        for rel in obj.relationships.iter_mut(){
            rel.object_id = rel.object_id.replace(":", replace);
            rel.qualifier = rel.qualifier.replace(":", replace);
        }
    }
    for ev in res.events.iter_mut(){
        //ev.id = ev.id.replace(":", replace);
        //ev.event_type = ev.event_type.replace(":", replace);
        for rel in ev.relationships.iter_mut(){
            rel.object_id = rel.object_id.replace(":", replace);
            rel.qualifier = rel.qualifier.replace(":", replace);
        }
    }
    for ot in res.object_types.iter_mut(){
        ot.name = ot.name.replace(":", replace);
    }
    res
}

pub fn ocpn_preprocessing(ocpn:OCPN) -> OCPN {
    let mut res: OCPN = ocpn.clone();

    let replace = "";

    //remove all ":" from object to type, places, tranistions, arcs, initial markings

    let mut new_object_to_type: HashMap<String, String> = HashMap::new();
    for (obj, typ) in res.object_to_type.iter(){
        new_object_to_type.insert(obj.replace(":", replace), typ.replace(":", replace));
    }
    res.object_to_type = new_object_to_type;

    for (_, place) in res.places.iter_mut(){
        place.object_type = place.object_type.replace(":", replace);
    }

    for (_, transition) in res.transitions.iter_mut(){
        transition.input_places.iter_mut().for_each(|(_, ot)| { *ot = ot.replace(":", replace); });
        transition.output_places.iter_mut().for_each(|(_, ot)| { *ot = ot.replace(":", replace); });

        //replace " " with "_"
        transition.activity = transition.activity.replace(" ", "_");
    }

    for (_, s_transition) in res.silent_transtions.iter_mut(){
        s_transition.input_places.iter_mut().for_each(|(_, ot)| { *ot = ot.replace(":", replace); });
        s_transition.output_places.iter_mut().for_each(|(_, ot)| { *ot = ot.replace(":", replace); });
    }

    for arc in res.arcs.iter_mut(){
        arc.object_type = arc.object_type.replace(":", replace);
    }

    let mut new_initial_marking: Marking = HashMap::new();
    if let Some(initial_marking) = &res.initial_marking {
        for (place, objects) in initial_marking.iter(){
            let new_objects = objects.iter().map(|o| o.replace(":", replace)).collect();
            new_initial_marking.insert(place.clone(), new_objects);
        }
        res.initial_marking = Some(new_initial_marking);
    }


    res
}


use serde_json::{json, Value};
use std::fs;


pub fn export_ocel_to_jsonocel_file(ocel: &OCEL, path: &str) -> std::io::Result<()> {
    let json_str = export_ocel_to_jsonocel(ocel);
    fs::write(path, json_str)
}


fn export_ocel_to_jsonocel(ocel: &OCEL) -> String {
    let mut objects_map = serde_json::Map::new();
    for obj in &ocel.objects {
        let mut obj_entry = serde_json::Map::new();
        obj_entry.insert("ocel:type".to_string(), Value::String(obj.object_type.clone()));

        // immer ein ocel:ovmap einfügen (auch wenn leer)
        let mut ovmap = serde_json::Map::new();
        for a in &obj.attributes {
            ovmap.insert(a.name.clone(), oc_attr_value_to_json(&a.value));
        }
        obj_entry.insert("ocel:ovmap".to_string(), Value::Object(ovmap));

        if !obj.relationships.is_empty() {
            let rels = obj.relationships.iter().map(|r| {
                json!({
                    "objectId": r.object_id,
                    "qualifier": r.qualifier
                })
            }).collect();
            obj_entry.insert("ocel:relationships".to_string(), Value::Array(rels));
        }

        objects_map.insert(obj.id.clone(), Value::Object(obj_entry));
    }

    let mut events_map = serde_json::Map::new();
    for ev in &ocel.events {
        let mut ev_entry = serde_json::Map::new();
        ev_entry.insert("ocel:activity".to_string(), Value::String(ev.event_type.clone()));
        ev_entry.insert("ocel:timestamp".to_string(), Value::String(ev.time.to_rfc3339()));

        let omap: Vec<Value> = ev.relationships
            .iter()
            .map(|r| Value::String(r.object_id.clone()))
            .collect();
        ev_entry.insert("ocel:omap".to_string(), Value::Array(omap));


        let mut vmap = serde_json::Map::new();
        for a in &ev.attributes {
            vmap.insert(a.name.clone(), oc_attr_value_to_json(&a.value));
        }
        ev_entry.insert("ocel:vmap".to_string(), Value::Object(vmap));

        events_map.insert(ev.id.clone(), Value::Object(ev_entry));
    }

    let mut object_types_map = serde_json::Map::new();
    for ot in &ocel.object_types {
        let mut ot_entry = serde_json::Map::new();
        if !ot.attributes.is_empty() {
            let mut at_map = serde_json::Map::new();
            for a in &ot.attributes {
                at_map.insert(a.name.clone(), Value::String(a.value_type.clone()));
            }
            ot_entry.insert("ocel:attributes".to_string(), Value::Object(at_map));
        }
        object_types_map.insert(ot.name.clone(), Value::Object(ot_entry));
    }

    let mut event_types_map = serde_json::Map::new();
    for et in &ocel.event_types {
        let mut et_entry = serde_json::Map::new();
        if !et.attributes.is_empty() {
            let mut at_map = serde_json::Map::new();
            for a in &et.attributes {
                at_map.insert(a.name.clone(), Value::String(a.value_type.clone()));
            }
            et_entry.insert("ocel:attributes".to_string(), Value::Object(at_map));
        }
        event_types_map.insert(et.name.clone(), Value::Object(et_entry));
    }


    let mut unique_types: HashSet<String> = HashSet::new();
    for ot in &ocel.object_types {
        unique_types.insert(ot.name.clone());
    }

    let object_type_names: Vec<Value> = unique_types
        .into_iter()
        .map(Value::String)
        .collect();

    let mut root = serde_json::Map::new();
    root.insert("ocel:global-log".to_string(), json!({
        "ocel:attribute-names": [],
        "ocel:object-types": object_type_names,
        "ocel:version": "1.0",
        "ocel:ordering": "timestamp"
    }));
    root.insert("ocel:global-event".to_string(), json!({
        "ocel:activity": "__INVALID__"
    }));
    root.insert("ocel:global-object".to_string(), json!({
        "ocel:type": "__INVALID__"
    }));


    root.insert("ocel:objects".to_string(), Value::Object(objects_map));
    root.insert("ocel:events".to_string(), Value::Object(events_map));
    root.insert("ocel:objectTypes".to_string(), Value::Object(object_types_map));
    root.insert("ocel:eventTypes".to_string(), Value::Object(event_types_map));

    serde_json::to_string_pretty(&Value::Object(root)).unwrap()
}


fn oc_attr_value_to_json(v: &OCELAttributeValue) -> Value {
    match v {
        OCELAttributeValue::Boolean(b) => Value::Bool(*b),
        OCELAttributeValue::Integer(i) => Value::Number((*i).into()),
        OCELAttributeValue::Float(f) => {
            serde_json::Number::from_f64(*f).map(Value::Number).unwrap_or(Value::Null)
        }
        OCELAttributeValue::String(s) => Value::String(s.clone()),
        _ => { Value::Null }
    }
}




use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::path::Path;


#[derive(Debug, Serialize, Deserialize)]
struct RuntimeInfo {
    time: f64,
    percentage: f32,
}


#[derive(Debug, Serialize, Deserialize)]
struct Run {
    id: usize,
    ocel_name: String,
    total_time: f64,
    runtime: HashMap<String, RuntimeInfo>,
    timestamp: String,
    precision: Option<f64>,
    fitness: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AnalyticsLog {
    runs: Vec<Run>,
}


pub fn save_analytics_2(
    path: &str,
    starting_times: &Vec<Instant>,
    steps_as_strings: &Vec<String>,
    endtime: Instant,
) {

    if cfg!(debug_assertions) {


        println!("################################################");
        println!("# Warning: DEBUG mode is active! Analytics will NOT be saved. #");
        println!("################################################");


        return;
    }else if !SAVE{
        println!("################################################");
        println!("# Warning: Saving analytics is disabled! Analytics will NOT be saved. #");
        println!("################################################");
        return;
    }

    println!("Saving analytics log to {}", path);

    //load JSON
    let file_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/BA_analytics.json";
    let old_content = read_to_string(file_path).unwrap_or_else(|_| "{}".to_string());
    let mut analytics: AnalyticsLog =
        from_str(&old_content).unwrap_or_else(|_| AnalyticsLog::default());

    let run_id = analytics.runs.len() + 1;
    let ocel_name = Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    //calc times
    let mut task_time_map: Vec<(String, f64)> = Vec::new();
    for i in 1..starting_times.len() {
        let elapsed_time = (starting_times[i] - starting_times[i - 1]) / NUMBER_OF_ITERATIONS as u32;
        task_time_map.push((steps_as_strings[i - 1].clone(), elapsed_time.as_secs_f64()));
    }

    //sort
    task_time_map.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    //calc percentages
    let sum: f64 = task_time_map.iter().map(|(_, t)| *t).sum();
    let mut percentages: Vec<(String, f32)> = Vec::new();
    for (name, time) in &task_time_map {
        percentages.push((name.clone(), (*time / sum * 100.09) as f32));
    }

    //build runtime map
    let mut runtime: HashMap<String, RuntimeInfo> = HashMap::new();
    for (name, time) in &task_time_map {
        let perc = percentages
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        runtime.insert(name.clone(), RuntimeInfo { time: *time, percentage: perc });
    }

    //calc total time
    let total_time = (endtime - starting_times[0]).as_secs_f64();
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let new_run = Run {
        id: run_id,
        ocel_name,
        total_time,
        runtime,
        timestamp,
        precision: None,
        fitness: None,
    };

    //insert new run
    analytics.runs.push(new_run);

    //write JSON
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path).unwrap();
    let json_str = to_string_pretty(&analytics).unwrap();
    file.write_all(json_str.as_bytes()).unwrap();
}







use std::error::Error;
use crate::enabled_log_act::{_get_contexts_and_bindings, get_event_presets};

fn analyze_ocel_file(path: &Path, filter:(i32, usize, i32)) -> Result<(usize, usize, usize, usize, usize, usize, usize, f32, usize, f32, f32), Box<dyn Error>> {

    if path.ends_with(".jsonocel") {
       return analyze_ocel1_file(Path::new(path), filter);
    }


    let mut ocel = import_ocel(path.to_str().unwrap());
    ocel = ocel_preprocessing(ocel);
    let object_to_type:HashMap<String, String> = ocel.objects.iter().map(|o| (o.id.clone(), o.object_type.clone())).collect();
    ocel = filter_ocel(ocel, object_to_type.clone(), filter.0, filter.1, filter.2);


    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;


    let num_events = ocel.events.len();


    let mut event_types:HashSet<String> = ocel.event_types.iter().map(|o| (o.name.clone())).collect();

    let num_event_types = event_types.len();


    let num_objects = ocel.objects.len();


    let mut object_types:HashSet<String> = ocel.object_types.iter().map(|o| (o.name.clone())).collect();
    let num_object_types = object_types.len();


    let events_ids: Vec<String> = ocel.events.iter().map(|event| event.id.clone()).collect();
    let all_object_types: HashSet<String> =
        ocel.object_types.iter().map(|ot| ot.name.clone()).collect();
    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    println!(".");
    let (presets, _) = enabled_log_act::get_event_presets(&ocel, &id_to_event);
    println!(".");

    let mut traces:Vec<Vec<OCELEvent>> = Vec::new();
    for (index, (event_id, preset)) in presets.iter().enumerate() {
        let event = id_to_event.get(event_id).unwrap();
        let trace = traces.iter().find(|t| t.contains(event));
        if !trace.is_none() {
            //check if trace is a subset if preset
            let trace = trace.unwrap();
            if is_superset(&preset.iter().map(|e| e.id.clone()).collect(), &trace.iter().map(|e| e.id.clone()).collect()) {
                //replace this trace with preset
                let mut events = preset.clone();
                events.push(event.clone());
                traces[index] = preset.clone();
            }
        }else{
            //add preset as new trace
            let mut events = preset.clone();
            events.push(event.clone());
            traces.push(events);
        }
    }



    let trace_lengths: Vec<usize> = traces
        .iter()
        .map(|b| b.len())
        .filter(|&len| len > 0)
        .collect();
    let trace_length_max = *trace_lengths.iter().max().unwrap_or(&0);
    let trace_length_min = *trace_lengths.iter().min().unwrap_or(&0);
    let trace_length_avg = if !trace_lengths.is_empty() {
        trace_lengths.iter().sum::<usize>() as f64 / trace_lengths.len() as f64
    } else {
        0.0
    };


    let mut objects_per_trace: Vec<HashMap<String, HashSet<String>>> = Vec::new();
    for trace in traces.iter(){

        let mut objects_in_trace: HashMap<String, HashSet<String>> = HashMap::new();
        for event in trace.iter(){
            for rel in event.relationships.iter(){
                let ot = object_to_type.get(&rel.object_id.clone()).unwrap();
                objects_in_trace.entry(ot.clone()).or_insert(HashSet::new()).insert(rel.object_id.clone());
            }
        }
        objects_per_trace.push(objects_in_trace);
    }
    //number of max objects with same object type per trace
    let mut num_max_objects_per_trace:Vec<usize> = objects_per_trace.iter()
        .map(|obj| obj.iter()
            .map(|(ot, objs)| objs.len()).max().unwrap_or(0)).collect();
    let max_num_objects_in_trace = *num_max_objects_per_trace.iter().max().unwrap_or(&0);
    let avg_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        num_max_objects_per_trace.iter().sum::<usize>() as f64 / num_max_objects_per_trace.len() as f64
    } else {
        0.0
    };
    let median_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        let mut sorted = num_max_objects_per_trace.clone();
        sorted.sort();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    } else {
        0.0
    };


    Ok((num_events, num_event_types, num_objects, num_object_types, traces.len(), trace_length_max, trace_length_min, trace_length_avg as f32, max_num_objects_in_trace, avg_num_objects_in_trace as f32, median_num_objects_in_trace as f32))
}

pub fn analyze_ocel1_file(path: &Path, filter: (i32, usize, i32)) -> Result<(usize, usize, usize, usize, usize, usize, usize, f32, usize, f32, f32), Box<dyn Error>> {
    println!("Analyzing OCEL 1.0 file");

    let mut ocel = import_ocel(path.to_str().unwrap());
    ocel = ocel_preprocessing(ocel);
    let object_to_type:HashMap<String, String> = ocel.objects.iter().map(|o| (o.id.clone(), o.object_type.clone())).collect();
    ocel = filter_ocel(ocel, object_to_type.clone(), filter.0, filter.1, filter.2);




    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;


    let num_events = ocel.events.len();


    let mut event_types:HashSet<String> = ocel.event_types.iter().map(|o| (o.name.clone())).collect();
    let num_event_types = event_types.len();


    let num_objects = ocel.objects.len();


    let mut object_types:HashSet<String> = ocel.object_types.iter().map(|o| (o.name.clone())).collect();
    let num_object_types = object_types.len();



    let events_ids: Vec<String> = ocel.events.iter().map(|event| event.id.clone()).collect();
    let all_object_types: HashSet<String> =
        ocel.object_types.iter().map(|ot| ot.name.clone()).collect();
    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let (presets, id_preset) = enabled_log_act::get_event_presets(&ocel, &id_to_event);


    let mut traces:Vec<Vec<OCELEvent>> = Vec::new();
    for (index, (event_id, preset)) in presets.iter().enumerate() {
        let event = id_to_event.get(event_id).unwrap();
        let trace = traces.iter().find(|t| t.contains(event));
        if !trace.is_none() {
            //check if trace is a subset if preset
            let trace = trace.unwrap();
            if is_superset(&preset.iter().map(|e| e.id.clone()).collect(), &trace.iter().map(|e| e.id.clone()).collect()) {
                //replace this trace with preset
                let mut events = preset.clone();
                events.push(event.clone());
                traces[index] = preset.clone();
            }
        }else{
            //add preset as new trace
            let mut events = preset.clone();
            events.push(event.clone());
            traces.push(events);
        }
    }


    let trace_lengths: Vec<usize> = traces
        .iter()
        .map(|b| b.len())
        .filter(|&len| len > 0)
        .collect();
    let trace_length_max = *trace_lengths.iter().max().unwrap_or(&0);
    let trace_length_min = *trace_lengths.iter().min().unwrap_or(&0);
    let trace_length_avg = if !trace_lengths.is_empty() {
        trace_lengths.iter().sum::<usize>() as f64 / trace_lengths.len() as f64
    } else {
        0.0
    };

    let mut objects_per_trace: Vec<HashMap<String, HashSet<String>>> = Vec::new();
    for trace in traces.iter(){

        let mut objects_in_trace: HashMap<String, HashSet<String>> = HashMap::new();
        for event in trace.iter(){
            for rel in event.relationships.iter(){
                let ot = object_to_type.get(&rel.object_id.clone()).unwrap();
                objects_in_trace.entry(ot.clone()).or_insert(HashSet::new()).insert(rel.object_id.clone());
            }
        }
        objects_per_trace.push(objects_in_trace);
    }
    //number of max objects with same object type per trace
    let mut num_max_objects_per_trace:Vec<usize> = objects_per_trace.iter()
        .map(|obj| obj.iter()
            .map(|(ot, objs)| objs.len()).max().unwrap_or(0)).collect();
    let max_num_objects_in_trace = *num_max_objects_per_trace.iter().max().unwrap_or(&0);
    let avg_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        num_max_objects_per_trace.iter().sum::<usize>() as f64 / num_max_objects_per_trace.len() as f64
    } else {
        0.0
    };
    let median_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        let mut sorted = num_max_objects_per_trace.clone();
        sorted.sort();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    } else {
        0.0
    };


    Ok((num_events, num_event_types, num_objects, num_object_types, traces.len(), trace_length_max, trace_length_min, trace_length_avg as f32, max_num_objects_in_trace, avg_num_objects_in_trace as f32, median_num_objects_in_trace as f32))
}

pub fn analyze_ocel2_file_xml(path: &Path, filter: (i32, usize, i32)) -> Result<(usize, usize, usize, usize, usize, usize, usize, f32, usize, f32, f32), Box<dyn Error>>{

    let mut ocel = process_mining::import_ocel_xml_file(path.to_str().unwrap());
    ocel = clean_ocel(ocel);
    ocel = ocel_preprocessing(ocel);
    let object_to_type:HashMap<String, String> = ocel.objects.iter().map(|o| (o.id.clone(), o.object_type.clone())).collect();
    ocel = filter_ocel(ocel, object_to_type.clone(), filter.0, filter.1, filter.2);

    let num_events = ocel.events.len();
    let num_event_types = ocel.event_types.len();
    let num_objects = ocel.objects.len();
    let num_object_types = ocel.object_types.len();

    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let (presets, id_preset) = enabled_log_act::get_event_presets(&ocel, &id_to_event);


    let mut traces:Vec<Vec<OCELEvent>> = Vec::new();
    for (index, (event_id, preset)) in presets.iter().enumerate() {
        let event = id_to_event.get(event_id).unwrap();
        let trace = traces.iter().find(|t| t.contains(event));
        if !trace.is_none() {
            //check if trace is a subset if preset
            let trace = trace.unwrap();
            if is_superset(&preset.iter().map(|e| e.id.clone()).collect(), &trace.iter().map(|e| e.id.clone()).collect()) {
                //replace this trace with preset
                let mut events = preset.clone();
                events.push(event.clone());
                traces[index] = preset.clone();
            }
        }else{
            //add preset as new trace
            let mut events = preset.clone();
            events.push(event.clone());
            traces.push(events);
        }
    }

    for trace in traces.iter_mut() {
        if trace.is_empty() {
            panic!("sss");
        }
    }


    let trace_lengths: Vec<usize> = traces
        .iter()
        .map(|b| b.len())
        .filter(|&len| len > 0)
        .collect();
    let trace_length_max = *trace_lengths.iter().max().unwrap_or(&0);
    let trace_length_min = *trace_lengths.iter().min().unwrap_or(&0);
    let trace_length_avg = if !trace_lengths.is_empty() {
        trace_lengths.iter().sum::<usize>() as f64 / trace_lengths.len() as f64
    } else {
        0.0
    };



    let mut objects_per_trace: Vec<HashMap<String, HashSet<String>>> = Vec::new();
    for trace in traces.iter(){

        let mut objects_in_trace: HashMap<String, HashSet<String>> = HashMap::new();
        for event in trace.iter(){
            for rel in event.relationships.iter(){
                let ot = object_to_type.get(&rel.object_id.clone()).unwrap();
                objects_in_trace.entry(ot.clone()).or_insert(HashSet::new()).insert(rel.object_id.clone());
            }
        }
        if objects_in_trace.is_empty() {
            panic!("sss");
        }
        objects_per_trace.push(objects_in_trace);
    }
    //number of max objects with same object type per trace
    let mut num_max_objects_per_trace:Vec<usize> = objects_per_trace.iter()
        .map(|obj| obj.iter()
            .map(|(ot, objs)| objs.len()).max().unwrap_or(0)).collect();
    let max_num_objects_in_trace = *num_max_objects_per_trace.iter().max().unwrap_or(&0);
    let avg_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        num_max_objects_per_trace.iter().sum::<usize>() as f64 / num_max_objects_per_trace.len() as f64
    } else {
        0.0
    };
    let median_num_objects_in_trace = if !num_max_objects_per_trace.is_empty() {
        let mut sorted = num_max_objects_per_trace.clone();
        sorted.sort();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    } else {
        0.0
    };

    Ok((num_events, num_event_types, num_objects, num_object_types, traces.len(), trace_length_max, trace_length_min, trace_length_avg as f32, max_num_objects_in_trace, avg_num_objects_in_trace as f32, median_num_objects_in_trace as f32))


}



/// Analysiert alle OCEL-JSON-Dateien in einem Ordner und speichert das Ergebnis als CSV

pub fn analyze_ocel_folder(folder: &str, output_file: &str, filter:(i32, usize, i32)) -> Result<(), Box<dyn Error>> {

    println!("Analyzing OCEL folder: {}", folder);

    let mut results = Vec::new();

    // Header
   let header = format!(
       "{:<40} | {:>12} | {:>12} | {:>12} | {:>15} | {:>15} | {:>15} | {:>12} | {:>12} | {:>20} | {:>20} | {:>20}",
       "File", "Events", "Event-Types", "Objects", "Object-Types", "Traces", "Trace-len-Max", "Trace-len-Min", "Trace-len-Avg", "Max objs per binding", "Avg objs per binding", "Median objs per binding"
   );
    let separator = "-".repeat(header.len());
    results.push(header.clone());
    results.push(separator.clone());

    // Durch alle JSON-Dateien gehen
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
            let (ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr) = analyze_ocel_file(&path, filter.clone())?;
            let fname = path.file_name().unwrap().to_string_lossy();

            // Formatierte Zeile
            let line = format!(
                "{:<40} | {:>12} | {:>12} | {:>12} | {:>15} | {:>15} | {:>15} | {:>12} | {:>12} | {:>20} | {:>20} | {:>20}",
                fname, ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr
            );
            results.push(line);
        }else if path.is_file() && path.extension().map(|e| e == "jsonocel").unwrap_or(false) {
            let (ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr) = analyze_ocel1_file(&path, filter.clone())?;
            let fname = path.file_name().unwrap().to_string_lossy();

            // Formatierte Zeile
            let line = format!(
                "{:<40} | {:>12} | {:>12} | {:>12} | {:>15} | {:>15} | {:>15} | {:>12} | {:>12} | {:>20} | {:>20} | {:>20}",
                fname, ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr
            );
            results.push(line);
        }else if path.is_file() && path.extension().map(|e| e == "xml").unwrap_or(false) {
            let (ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr) = analyze_ocel2_file_xml(&path, filter.clone())?;
            let fname = path.file_name().unwrap().to_string_lossy();

            // Formatierte Zeile
            let line = format!(
                "{:<40} | {:>12} | {:>12} | {:>12} | {:>15} | {:>15} | {:>15} | {:>12} | {:>12} | {:>20} | {:>20} | {:>20}",
                fname, ne, net, no, not, nt, tmax, tmin, tav, max_ob_tr, avg_b_tr, med_ob_tr
            );
            results.push(line);
        }
    }

    // In Datei schreiben
    let out = results.join("\n");
    fs::write(output_file, out)?;

    println!("Ergebnisse gespeichert in {}", output_file);
    Ok(())
}


pub fn get_ocpn_path(file_path: &str, folder_path: &str) -> String {
    // get name of the file from the path
    let file_name = Path::new(file_path)
        .file_stem().unwrap()
        .to_string_lossy();

    // create new name with ocpn_ prefix
    let new_name = format!("ocpn_{}.json", file_name);

    let full_path = format!("{}/{}", folder_path.trim_end_matches('/'), new_name);

    if Path::new(&full_path).exists() {
        full_path
    } else {
        panic!("OCPN file could not be found at path: {}", full_path);
    }
}


pub fn import_ocel(file_path: &str) -> OCEL{
    //check if it is jsonocel or json file
    if file_path.ends_with(".jsonocel") {
        import_ocel_from_jsonocel(file_path)
    } else if file_path.ends_with(".json") {
        import_ocel_from_json_file(file_path)
    } else {
        panic!("File format not supported: {}", file_path);
    }
}



pub fn filter_ocel (mut ocel:OCEL, object_to_type:HashMap<String, String>, max_events:i32, max_objects_per_trace:usize, max_trace_length:i32) -> OCEL{



    if max_events != -1{
        //filter events
        //sort events by time
        ocel.events.sort_by(|a, b| a.time.cmp(&b.time));

        let num_events = max_events as usize;
        //only leave first num_events events
        if ocel.events.len() > num_events {
            ocel.events = ocel.events[..num_events].to_vec();
        }
    }



    println!("filter_ocel");
    let id_to_event: HashMap<String, OCELEvent> = ocel
        .events
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let (presets, _) = enabled_log_act::get_event_presets(&ocel, &id_to_event.clone());
    println!("get traces");
    let mut traces:Vec<Vec<OCELEvent>> = Vec::new();
    for (index, (event_id, preset)) in presets.iter().enumerate() {
        let event = id_to_event.get(event_id).unwrap();
        let trace = traces.iter().find(|t| t.contains(event));
        if !trace.is_none() {
            //check if trace is a subset if preset
            let trace = trace.unwrap();
            if is_superset(&preset.iter().map(|e| e.id.clone()).collect(), &trace.iter().map(|e| e.id.clone()).collect()) {
                //replace this trace with preset
                traces[index] = preset.clone();
            }
        }else{
            //add preset as new trace
            traces.push(preset.clone());
        }
    }
    println!(".");
    println!("traces max: {:?}", traces.iter().map(|t| t.len()).max());

    //first apprach: filter every trace that has too many objects
    let mut filtered_events_out: HashSet<String> = HashSet::new();
    for trace in traces.iter(){

        let mut objects_in_trace: HashMap<String, HashSet<String>> = HashMap::new();
        'a: for event in trace.iter(){

            for rel in event.relationships.iter(){

                //only filter events
                if objects_in_trace.get(object_to_type.get(&rel.object_id.clone()).unwrap()) != None 
                    && objects_in_trace.get(object_to_type.get(&rel.object_id.clone()).unwrap()).unwrap().len() >= max_objects_per_trace {

                    filtered_events_out.insert(event.id.clone());
                    continue 'a;
                }

                objects_in_trace.entry(object_to_type.get(&rel.object_id.clone()).unwrap().clone()).or_insert_with(HashSet::new).insert(rel.object_id.clone());
            }
        }
        /*
        //filter whole trace
        if objects_in_trace.iter().any(|(k, v)| v.len() > max_objects_per_trace) {
            for event in trace.iter(){
                filtered_events_out.insert(event.id.clone());
            }
        }

         */
    }

    if max_trace_length != -1 {
        let length:usize = max_trace_length as usize;
        //cut traces that are too long
        for trace in traces.iter(){
            if trace.len() > length {
                for event in trace.iter().skip(length) {
                    filtered_events_out.insert(event.id.clone());
                }
            }
        }
    }



    println!("Filtered events: {:?}", filtered_events_out.len());
    println!("Percent: {:?}", filtered_events_out.len() as f32 / ocel.events.len() as f32 * 100.0);
    println!("Remaining events: {:?}", ocel.events.len() - filtered_events_out.len());

    ocel.events = ocel.events.into_iter().filter(|e| !filtered_events_out.contains(&e.id)).collect();

    ocel
}


use std::path::PathBuf;
use crate::evaluator::apply;

#[derive(Debug, Deserialize)]
struct RunsFile {
    runs: Vec<Run>,
}
pub fn auto_eval(){
    println!("Starting evaluation...");

    // Pfade
    let json_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/BA_analytics.json";
    let logs_dir = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCEL";
    let ocpns_dir = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCPN";

    // JSON laden
    let data = fs::read_to_string(json_path).expect("Error reading the JSON file");
    let parsed: RunsFile = serde_json::from_str(&data).expect("Error parsing the JSON file");




    // Zählen
    let mut counts: HashMap<String, usize> = HashMap::new();
    for run in &parsed.runs {
        *counts.entry(run.ocel_name.clone()).or_insert(0) += 1;
    }

    println!("count of all ocel_names:");
    for (name, count) in &counts {
        println!("{}: {}", name, count);
    }

    // Insert each OCEL from OCEL directory with count = 5
    let mut logs_to_do: HashMap<String, i32> = HashMap::new();
    for entry in fs::read_dir(logs_dir).expect("read dir failed") {
        let entry = entry.expect("Error reading entry");
        if entry.file_type().expect("ERROR").is_file() {
            let file_name = entry.file_name().into_string().unwrap();
            logs_to_do.insert(file_name, 5); // Counter = 5 initialisieren
        }
    }
    for run in &parsed.runs {
        if let Some(counter) = logs_to_do.get_mut(&run.ocel_name) {
            *counter -= 1;
        } else {
            println!("Warning: '{}' is not in directory!", run.ocel_name);
        }
    }

    println!("logs to do:");
    println!("{:?}", logs_to_do);

    while true{
        // Pfade und "run"
        for (log, c) in logs_to_do.clone() {
            println!("Run {:?} ...", log);

            if [
                "order_management.jsonocel",
                "sap_transfer_order.jsonocel",

            ].contains(&log.as_str()) {
                println!("skipping {} for debug", log);
                continue;
            }


            let mut log_path = PathBuf::from(logs_dir);
            log_path.push(log.clone());

            let mut ocpn_path = PathBuf::from(ocpns_dir);
            ocpn_path.push(format!("ocpn_{}", log.replace(".jsonocel", ".json").replace(".xml", ".json")));



            if c > 0 {
                let mut ocel:OCEL;
                if log.ends_with(".xml") {
                    //Ocel 2.0
                    ocel = process_mining::import_ocel_xml_file(log_path.to_str().unwrap());
                } else {
                    //Ocel 1.0
                    ocel = utils::import_ocel(log_path.to_str().unwrap());
                }

                println!("    ->   {:?} ... {:?}", log_path, ocpn_path);

                let ocpn = ocpn_preprocessing(utils::import_from_json(
                    ocpn_path.to_str().unwrap(),
                ));

                let log_path_ = log_path.to_path_buf(); // falls log_path ein &Path ist

                let (f, p) = run_with_timeout(
                    move || {
                        apply(ocel, ocpn, Some(log_path_.to_str().unwrap()))
                    },
                    Duration::from_secs(10800), //3 hours timeout
                );

                //let (f,p) = apply(ocel, ocpn, Some(log_path.to_str().unwrap()));


                let data_ = fs::read_to_string(json_path).expect("Error reading the JSON file");
                let mut parsed_: Value = serde_json::from_str(&data_).expect("Error parsing the JSON file");

                if let Some(runs) = parsed_.get_mut("runs").and_then(|r| r.as_array_mut()) {
                    if let Some(last_run) = runs.last_mut() {
                        last_run["fitness"] = Value::Number(serde_json::Number::from_f64(f).unwrap());
                        last_run["precision"] = Value::Number(serde_json::Number::from_f64(p).unwrap());

                        fs::write(
                            json_path,
                            serde_json::to_string_pretty(&parsed_).expect("Error serializing JSON")
                        ).expect("Error writing JSON file");
                    }
                }


            } else {
                println!("Skipping {}, already done 5 times.", log);
            }

            logs_to_do.entry(log).and_modify(|e| *e -= 1);
        }
        if logs_to_do.values().all(|&v| v <= 0) {
            break;
        }
    }


}

pub fn apply_filter(filter:(i32, usize, i32)){

    let logs_dir = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCEL";

    let source_logs_dir = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/OCEL";

    print!(".");
    for entry in fs::read_dir(source_logs_dir).expect("read dir failed") {
        let entry = entry.expect("Reading entry failed");
        if entry.file_type().expect("ERROR").is_file() {
            let file_name = entry.file_name().into_string().unwrap();
            let mut source_path = PathBuf::from(source_logs_dir);
            source_path.push(file_name.clone());
            println!("source: {}", source_path.display());

            let mut target_path = PathBuf::from(logs_dir);
            target_path.push(file_name.clone());
            println!("target: {}\n", target_path.display());


            //DEBUG
            /*
            if Path::new(&target_path).exists() {
                println!("File already exists, skipping: {}", target_path.display());
                continue;
            }
             */

            let mut ocel: OCEL;

            if file_name.ends_with(".xml") {
                //Ocel 2.0
                ocel = process_mining::import_ocel_xml_file(source_path.to_str().unwrap());
                ocel = clean_ocel(ocel);
            } else {
                //Oce 1.0
                clean_ocel_ids_and_types_for_json(source_path.to_str().unwrap(), source_path.to_str().unwrap());
                ocel = import_ocel(source_path.to_str().unwrap());
                ocel = clean_ocel(ocel);
            }

            let object_to_type: HashMap<String, String> = ocel.objects.iter().map(|o| (o.id.clone(), o.object_type.clone())).collect();

            ocel = filter_ocel(ocel, object_to_type.clone(), filter.0, filter.1, filter.2);

            if file_name.ends_with(".xml") {
                //Ocel 2.0
                process_mining::export_ocel_xml_path(&ocel.clone(), target_path).expect("export failed");
            } else {
                //Ocel 1.0
                export_ocel_to_jsonocel_file(&ocel, target_path.to_str().unwrap()).unwrap()
            }
        }
    }
}

pub fn export_filtered_ocel(){
    let target_dir = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCEL";
    let sourcepath = "/Users/maxbaumeister/PycharmProjects/ocpa_2/sample_logs/ocel2/xmlocel/BA_Eval/obj-centr-log.jsonocel";
    let filter:(i32, usize, i32) = (-1, 5000, -1);


    let file_name = Path::new(sourcepath)
        .file_name().unwrap()
        .to_string_lossy().to_string();

    let mut target_path = PathBuf::from(target_dir);
    target_path.push(file_name.clone());


    let mut ocel: OCEL;

    if sourcepath.ends_with(".xml") {
        //Ocel 2.0
        ocel = process_mining::import_ocel_xml_file(sourcepath);
        ocel = clean_ocel(ocel);
    } else {
        //Ocel 1.0
        clean_ocel_ids_and_types_for_json(sourcepath, sourcepath);
        ocel = import_ocel(sourcepath);
        ocel = clean_ocel(ocel);
    }

    let object_to_type: HashMap<String, String> = ocel.objects.iter().map(|o| (o.id.clone(), o.object_type.clone())).collect();

    ocel = filter_ocel(ocel, object_to_type.clone(), filter.0, filter.1, filter.2);



    if sourcepath.ends_with(".xml") {
        //Ocel 2.0
        process_mining::export_ocel_xml_path(&ocel.clone(), target_path).expect("export failed");
    } else {
        //Ocel 1.0
        export_ocel_to_jsonocel_file(&ocel, target_path.to_str().unwrap()).unwrap()
    }
}


use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use rustworkx_core::petgraph::visit::NodeRef;

fn run_with_timeout<F>(func: F, timeout: Duration) -> (f64, f64)
where
    F: FnOnce() -> (f64, f64) + Send + 'static,
{
    println!("Running with timeout {:?}", timeout);
    
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = func();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(res) => res,
        Err(_) => (2.0, 2.0),
    }
}






pub fn analyze_runs(
    json_path: &str,
) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(json_path)?;
    let data: Value = serde_json::from_str(&json_str)?;

    let runs = data["runs"].as_array().ok_or("Missing runs array")?;

    let mut grouped: HashMap<String, Vec<&Value>> = HashMap::new();
    for run in runs {
        let name = run["ocel_name"].as_str().unwrap_or("UNKNOWN").to_string();
        grouped.entry(name).or_default().push(run);
    }

    let mut results: HashMap<String, Value> = HashMap::new();

    for (name, runs) in grouped {
        let mut runtime_categories: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();
        let mut fitness_values = Vec::new();
        let mut precision_values = Vec::new();
        let mut total_times = Vec::new();

        for run in &runs {
            if let Some(runtime) = run["runtime"].as_object() {
                for (category, values) in runtime {
                    let time = values["time"].as_f64().unwrap_or(0.0);
                    let perc = values["percentage"].as_f64().unwrap_or(0.0);

                    runtime_categories
                        .entry(category.clone())
                        .or_default()
                        .0
                        .push(time);
                    runtime_categories
                        .entry(category.clone())
                        .or_default()
                        .1
                        .push(perc);
                }
            }
            fitness_values.push(run["fitness"].as_f64().unwrap_or(0.0));
            precision_values.push(run["precision"].as_f64().unwrap_or(0.0));
            total_times.push(run["total_time"].as_f64().unwrap_or(0.0));
        }

        let mut categories = serde_json::Map::new();
        for (cat, (times, percentages)) in runtime_categories {
            let avg_time = mean(&times);
            let avg_percentage = mean(&percentages);
            let max_dev = percentages.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - percentages.iter().cloned().fold(f64::INFINITY, f64::min);

            categories.insert(
                cat,
                serde_json::json!({
                    "avg_time": avg_time,
                    "avg_percentage": avg_percentage,
                    "max_deviation_percent": max_dev
                }),
            );
        }

        let avg_total_time = mean(&total_times);

        results.insert(
            name,
            serde_json::json!({
                "count": runs.len(),
                "avg_total_time": avg_total_time,
                "categories": categories,
                "fitness": fitness_values,
                "precision": precision_values
            }),
        );
    }
    println!("{:#?}", results);
    Ok(results)
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}


pub(crate) fn analyize_ocpn(){
    let ocpn_path = "/Users/maxbaumeister/RustroverProjects/Fitness-and-Precision-for-OCPM/src/test/Data/BA_Evaluation_final/BA_Evaluation_final_filtered/OCPN";

    let mut files = Vec::new();

    for entry in fs::read_dir(Path::new(ocpn_path)).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }

    for file in files {
        let ocpn = import_from_json(&file);
        println!("File: {}", file);
        println!("Number of places: {}", ocpn.places.len());
        println!("Number of transitions: {}", ocpn.transitions.len());
        println!("Number of arcs: {}", ocpn.arcs.len());
        println!("Number of silent transitions: {}", ocpn.silent_transtions.len());


        //calculate average degree of places and transitions
        let mut place_degrees: Vec<usize> = Vec::new();
        for place in &ocpn.places {
            let mut degree = 0;
            for arc in &ocpn.arcs {
                if arc.source == place.0.clone() || arc.target == place.0.clone()  {
                    degree += 1;
                }
            }
            place_degrees.push(degree);
        }
        let avg_place_degree = if !place_degrees.is_empty() {
            place_degrees.iter().sum::<usize>() as f64 / place_degrees.len() as f64
        } else {
            0.0
        };
        println!("Average place degree: {:.2}", avg_place_degree);

        let mut transition_degrees: Vec<usize> = Vec::new();
        for transition in &ocpn.transitions {
            let mut degree = 0;
            for arc in &ocpn.arcs {
                if arc.source == transition.0.clone()  || arc.target == transition.0.clone()  {
                    degree += 1;
                }
            }
            transition_degrees.push(degree);
        }
        let avg_transition_degree = if !transition_degrees.is_empty() {
            transition_degrees.iter().sum::<usize>() as f64 / transition_degrees.len() as f64
        } else {
            0.0
        };
        println!("Average transition degree: {:.2}", avg_transition_degree);

        //average num of silent transitions per place as input

        let mut silent_transitions_per_place: Vec<usize> = Vec::new();
        for place in &ocpn.places {
            let mut count = 0;
            for arc in &ocpn.arcs {
                if arc.source == place.0.clone() {
                    //find transition
                    let transition = ocpn.silent_transtions.iter().find(|t| t.0.clone() == arc.target);
                    if let Some(t) = transition {
                        count += 1;

                    }
                }
            }
            silent_transitions_per_place.push(count);
        }
        let avg_silent_transitions_per_place = if !silent_transitions_per_place.is_empty() {
            silent_transitions_per_place.iter().sum::<usize>() as f64 / silent_transitions_per_place.len() as f64
        } else {
            0.0
        };
        println!("Average silent transitions per place (as input): {:.2}", avg_silent_transitions_per_place);



        //average num of silent transitions per place as output

        let mut silent_transitions_per_place_output: Vec<usize> = Vec::new();
        for place in &ocpn.places {
            let mut count = 0;
            for arc in &ocpn.arcs {
                if arc.target == place.0.clone() {
                    //find transition
                    let transition = ocpn.silent_transtions.iter().find(|t| t.0.clone() == arc.source);
                    if let Some(t) = transition {
                        count += 1;

                    }
                }
            }
            silent_transitions_per_place_output.push(count);
        }
        let avg_silent_transitions_per_place_output = if !silent_transitions_per_place_output.is_empty() {
            silent_transitions_per_place_output.iter().sum::<usize>() as f64 / silent_transitions_per_place_output.len() as f64
        } else {
            0.0
        };
        println!("Average silent transitions per place (as output): {:.2}", avg_silent_transitions_per_place_output);

        println!();
    }
}


pub fn print_multiplicities(folder: &str){
    //for each file
    let mut output: Vec<String> = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let mut ocel:OCEL;
        let mut ocpn:OCPN;
        if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
            ocel = import_ocel_from_json_file(path.to_str().unwrap());
        }
        else if path.is_file() && path.extension().map(|e| e == "jsonocel").unwrap_or(false) {
            ocel = import_ocel(path.to_str().unwrap());
        }
        else if path.is_file() && path.extension().map(|e| e == "xml").unwrap_or(false) {
            ocel = import_ocel_xml_file(path.to_str().unwrap());
        }
        else {
            panic!("Error importing ocel path:{:?}", path);
        }

        let id_to_event: HashMap<String, OCELEvent> = ocel
            .events
            .iter()
            .map(|e| (e.id.clone(), e.clone()))
            .collect();
        if id_to_event.is_empty() {
            panic!("Error importing ocel path:{:?}", path);
        }
        let (presets, id_preset) = enabled_log_act::get_event_presets(&ocel, &id_to_event);

        let mut objects_to_type :HashMap<String, String> = HashMap::new();
        for object in ocel.objects.iter(){
            objects_to_type.insert(object.id.clone(), object.object_type.clone());
        }

        //get traces
        let mut traces:Vec<Vec<OCELEvent>> = Vec::new();
        for (index, (event_id, preset)) in presets.iter().enumerate() {
            let event = id_to_event.get(event_id).unwrap();
            let trace = traces.iter().find(|t| t.contains(event));
            if !trace.is_none() {
                //check if trace is a subset if preset
                let trace = trace.unwrap();
                if is_superset(&preset.iter().map(|e| e.id.clone()).collect(), &trace.iter().map(|e| e.id.clone()).collect()) {
                    //replace this trace with preset
                    let mut events = preset.clone();
                    events.push(event.clone());
                    traces[index] = events;
                }
            }else{
                //add preset as new trace
                let mut events = preset.clone();
                events.push(event.clone());
                traces.push(events);
            }
        }

        let mut multiplicities:Vec<HashMap<String, usize>> = Vec::new();

        for trace in traces {
            let mut mark:HashMap<String, usize> = HashMap::new();

            for event in trace{
                let objects = event.relationships;
                for object in objects{
                    let ot = objects_to_type.get(&object.object_id).unwrap().clone();
                    mark.entry(ot.clone()).and_modify(|e| *e += 1).or_insert(1);
                }
            }
            multiplicities.push(mark.clone());
        }

        let mut res:Vec<HashMap<String, usize>> = Vec::new();
        for m in multiplicities.iter(){
            if !res.contains(&m.clone()){
                res.push(m.clone());
            }
        }
        output.push(format!("Log:{:?} ; different multiplicities: {:?}", path.file_name().unwrap(), res.len()));


    }

    println!("{:#?}", output);
}











