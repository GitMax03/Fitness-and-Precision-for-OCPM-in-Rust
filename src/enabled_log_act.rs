extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use process_mining::ocel::ocel_struct::{OCELEvent};
use rustworkx_core::petgraph::prelude::*;
use rustworkx_core::traversal::ancestors;
use counter::Counter;


use crate::utils::*;
/*

- Determine the enabled log activities
    - Construct Event Object Graph
    - Extract Event Preset (for each event)
    - Calculate Prefix (for each Object)
    - Merge Prefix intro Context
    => Collect events with same context => all enables activities of context


- Determine the enabled model activities

 */
//TODO: not fully tested
pub fn construct_event_object_graph(ocel: &pm::OCEL) -> DiGraph<String, ()> {
    #[cfg(feature = "progress")]
    println!("constructing Event Object Graph (EOG) ...");
    
    //TODO: order of Events?

    let mut eog = DiGraph::<String, ()>::new();
    let mut id_to_index: HashMap<String, NodeIndex> = HashMap::new();
    
    //add all vertices
    for event in &ocel.events { 
        let id = event.id.clone();
        let node = eog.add_node(id.clone());
        id_to_index.insert(id, node);
    }

    //add all edges
    for i in 0..ocel.events.len() {
        //print %
        #[cfg(feature = "progress-percentage")]
        println!("EOG: {}%", (i as f32 / ocel.events.len() as f32 * 100.0));

        //TODO: make more efficient
        //collect all objects for event i
        let associated_objects = ocel.events[i].relationships.iter()
            .map(|o_id| o_id.object_id.clone())
            .collect::<Vec<String>>();

        //only consider events after event[i]
        for j in (i+1)..ocel.events.len() {
            //collect all objects for event j
            let associated_objects_j = ocel.events[j].relationships.iter()
                .map(|o_id| o_id.object_id.clone())
                .collect::<Vec<String>>();

            //if intersection of objects is not empty => add edge
            if !associated_objects.is_empty() && !associated_objects_j.is_empty() &&
                has_intersection(&associated_objects, &associated_objects_j) {
                //add edge to graph
                let node_i = id_to_index.get(&ocel.events[i].id).unwrap();
                let node_j = id_to_index.get(&ocel.events[j].id).unwrap();
                eog.add_edge(*node_i, *node_j, ());
            }
        }
    }
    eog
}

//TODO: not fully tested + return event ID?
pub fn get_event_presets(ocel: &pm::OCEL) -> HashMap<String, Vec<OCELEvent>> {
    //TODO: order important?
    #[cfg(feature = "progress")]
    println!("constructing presets ...");

    //get preset => all events that are connected to event_id in the eog
    //=> get all predecessors of event_id

    //TODO: does function change ocel?
    let eog = construct_event_object_graph(ocel);
    //key: id; value: preset
    let mut presets: HashMap<String, Vec<OCELEvent>> = HashMap::new();

    
    /*
    ancestors 
    - maybe calc in own code if no good lib available
    - maybe more efficient to use own code
    
    - use hash map to store already calculated presets
    - use petgraph::visit::Dfs???
    - reverse edges, check which node is reachable for each x
    - get x from reversed event_id list
    - calc preset recursively and store in hash map
     */

  
    //get presets
    for node in eog.node_indices() {
        //print %
        #[cfg(feature = "progress-percentage")]
        println!("Presets: {}%", (presets.len() as f32 / eog.node_count() as f32 * 100.0));


        let ancestors: Vec<NodeIndex> = ancestors(&eog, node).filter(|n| *n != node).collect();
        let event_id = eog[node].clone();
        //to get ancestors as event_ids and not NodeIndex
        let ancestors_str: Vec<String> = ancestors.iter().map(|n| eog[*n].clone()).collect();

        //get Event of event_ids
        let ancestors_events: Vec<OCELEvent> = ancestors_str.iter()
            .filter_map(|id| ocel.events.iter().find(|e| e.id == *id))
            .cloned()
            .collect();

        presets.insert(event_id.clone(), ancestors_events);
    }
    presets
}

//TODO: not fully tested
pub fn get_contexts_and_bindings (ocel: &pm::OCEL) -> (HashMap<String, HashMap<String, Counter<Vec<String>>>>,
                                                       HashMap<(String,String), HashMap<String, Vec<String>>>) {
    #[cfg(feature = "progress")]
    println!("constructing contexts and bindings...");

    //TODO: check if OCELType is object type
    //TODO: are eventType = activities? yes?
    //TODO: save EventType (activity) as String or OCELType?

    let object_types = ocel.object_types.clone();

    //HashMap [event_id, HashMap [object_type, Counter(activity/EventType)]]
    let mut contexts: HashMap<String, HashMap<String, Counter<Vec<String>>>> = HashMap::new();
    /*

    for each event, save all objects that are involved
                  HashMap [event_id, Vec [activity/EventType, HashMap [Objecttype, Vec<Objects>]]]
    TODO: maybe event_id or ther attributes are unecessary
     */
    let mut bindings: HashMap<(String,String), HashMap<String, Vec<String>>> = HashMap::new();
    
    let presets = get_event_presets(ocel);

    //object id to object type
    let objects_to_objecttypes: HashMap<String, String> = ocel.objects.iter()
        .map(|o| (o.id.clone(), o.object_type.clone()))
        .collect();

    //preset.keys(): event_id
    for event in ocel.events.clone() {
        //print %
        #[cfg(feature = "progress-percentage")]
        println!("Contexts and Bindings: {}%", (contexts.len() as f32 / ocel.events.len() as f32 * 100.0));
        
        let mut context: HashMap<String, Counter<Vec<String>>> = HashMap::new();

        
        //add all activities to bindings
        bindings.insert((event.id.clone(),event.event_type.clone()), object_types.iter().map(|ot| (ot.name.clone(), Vec::new())).collect());
        for associated_object in &event.relationships {
            //get object type
            let object_type = objects_to_objecttypes.get(&associated_object.object_id).unwrap();
            
            //add object to correct binding
            bindings.get_mut(&(event.id.clone(), event.event_type.clone())).expect("ERROR: Binding not found")
                .get_mut(&object_type.clone())
                .unwrap().push(associated_object.object_id.clone());
        }
        

        //context for this event
        for preset in presets.get(&event.id){

            //object type : (object_id, counter of activities)
            let mut associated_objects: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
            
            for e in preset{
                
                //for each e in preset, get all objects and their types
                for associated_object in &e.relationships {
                    
                    
                    //context
                    let object_type = objects_to_objecttypes.get(&associated_object.object_id).unwrap();
                    
                    //if object type is not in associated_objects, add it
                    if !associated_objects.contains_key(object_type) {
                       
                        let mut hash_map = HashMap::new();
                        hash_map.insert(associated_object.object_id.clone(), vec![e.event_type.clone()]);
                        associated_objects.insert(object_type.clone(), hash_map);
                    }else {
                        
                        //check if object is already in associated_objects
                        if !associated_objects.get_mut(object_type).unwrap().contains_key(&associated_object.object_id) {
                            
                            associated_objects.get_mut(object_type).unwrap().insert(associated_object.object_id.clone(), vec![e.event_type.clone()]);
                        } else {
                            //if object is already in associated_objects, increment the counter
                            associated_objects.get_mut(object_type).unwrap().get_mut(&associated_object.object_id).unwrap().push(e.event_type.clone());
                        }
                    }
                }
            }

            //add objects to context
            for (object_type, objects_map) in associated_objects {
                
                for (_, objects) in objects_map {

                    context
                        .entry(object_type.clone())
                        .or_insert_with(Counter::new)
                        .entry(objects)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                }
            }
        }
        //add context to contexts
        contexts.insert(event.id.clone(), context);
    }
    (contexts, bindings)
}

//hashing context
//TODO: maybe use serde_JSON: safer but slower
fn hash_context(context: &HashMap<String, Counter<Vec<String>>>) -> u64 {

    let mut hasher = DefaultHasher::new();
    //sort keys to ensure consistent hashing
    let mut sorted_keys: Vec<&String> = context.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        //hash key
        key.hash(&mut hasher);

        // extract counter keys
        if let Some(counter) = context.get(key) {
            // collect keys from the counter
            let mut vec_keys: Vec<String> = counter.iter().flat_map(|(k, _count)| k.clone()).collect::<Vec<String>>();
            // sort keys to ensure consistent hashing
            vec_keys.sort(); //TODO maybe whole different approach needed
            vec_keys.hash(&mut hasher);
        }
    }
    hasher.finish()
}

//TODO: efficiency: maybe use a more efficient way to get activities
fn get_activities(event_id: String, ocel: &pm::OCEL) -> Vec<String> {

    //get activities for event_id
    let activities: Vec<String> = ocel.events.iter()
        .filter(|e| e.id == event_id)
        .map(|e| e.event_type.clone()).collect();
    
    activities
}

//TODO: empty contexts problem 

//                                              context: HashMap [event_id, HashMap [object_type, Counter(activity/EventType)]]
pub fn get_enabled_log_activities(ocel: &pm::OCEL, contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>) 
    -> (HashMap<String, HashSet<String>>, HashMap<u64, (HashSet<String>, HashSet<String>)>) {///return: <event_id, Vec<activities>> , seen context (in raw format => formatting costs time)
    #[cfg(feature = "progress")]
    println!("getting enabled log activities ...");
    
    //assumption: each event is exaclty in one enables log activity entry !!!!
    

    //all duplicate contexts: Hashmap: hashed(context), (event_ids, activities)
    let mut seen_contexts: HashMap<u64, (HashSet<String>, HashSet<String>)> = HashMap::new();


    //enabled log activities: // HashMap [event_id, Vec<activity/EventType>]
    let mut enabled_log_activities: HashMap<String, HashSet<String>> = HashMap::new();

    //TODO can be done better???? must
    
    
    //for each event, get context and check if it is already seen
    for (event_id, context) in contexts{

        let hashed_context = hash_context(context);
        //if context is already seen, add event_id to seen_contexts
        let seen_context = seen_contexts
            .entry(hashed_context)
            .or_default();
        seen_context.0.insert(event_id.clone());
        //add activities to seen_contexts
        let activities = get_activities(event_id.clone(), ocel);
        seen_context.1.extend(activities.clone());
        //delete duplicates => since HashSet => no duplicates 
    }
    
    
    

    //get enabled log activities for each event_id
    for (_, (event_ids, activities)) in seen_contexts.clone() {

        for event_id in event_ids {

            //for each event_id, get activities
            *enabled_log_activities
                .entry(event_id.clone())
                .or_default() = activities.clone();
        }
    }
    (enabled_log_activities, seen_contexts)
}

