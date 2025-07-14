use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::fs::{OpenOptions, read_to_string};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use chrono::{Local, TimeZone};
use std::process::Command;
use std::time::Instant;
use counter::Counter;
use process_mining::OCEL;
use multiset::HashMultiSet;
use multiset_hash::RistrettoHash;
use process_mining::ocel::ocel_struct::{OCELAttributeValue, OCELEvent, OCELEventAttribute, OCELObject, OCELRelationship, OCELType, OCELTypeAttribute};
use crate::NUMBER_OF_ITERATIONS;
use crate::structs_for_ocpm::OCPN;


pub fn evaluate_runtime(starting_times:&Vec<Instant>, steps_as_strings:&Vec<String>) -> (Vec<(String, f64)>, Vec<(String, f32)>) {
    
    let endtime = std::time::Instant::now();
    
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

    let (task_time_map, percentages) = evaluate_runtime(starting_times, steps_as_strings);

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
    
  
    for (event_id, event_val) in json_value["ocel:events"].as_object().unwrap() {
        let mut event: OCELEvent;
        
        let mut relationships: Vec<OCELRelationship> = Vec::new();
        
        for object_id in event_val["ocel:omap"].as_array().unwrap() {
            relationships.append(vec![OCELRelationship{
                qualifier: objects_to_type.get(object_id.as_str().unwrap()).unwrap().to_string(),
                object_id: object_id.as_str().unwrap_or("").to_string(),
            }].as_mut());
        }
        
        activitys.insert(event_val["ocel:activity"].as_str().unwrap_or("").to_string());
        
        event = OCELEvent {
            id: event_id.clone(),
            event_type: event_val["ocel:activity"].as_str().unwrap_or("").to_string(),
            time: chrono::DateTime::parse_from_rfc3339(event_val["ocel:timestamp"].as_str().unwrap_or("")).unwrap_or_else(|_| chrono::FixedOffset::east_opt(0).unwrap().timestamp(0, 0)),
            attributes: empty_event_attribute.clone(),
            relationships,
        };
        events.push(event);
    }
    
    
    
    
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

