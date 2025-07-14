extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use process_mining::ocel::ocel_struct::{OCELEvent};
use rustworkx_core::traversal::ancestors;
use counter::Counter;
use multiset::HashMultiSet;
//use petgraph::graph::{DiGraph, NodeIndex};
use rustworkx_core::petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::NodeRef;
use process_mining::OCEL;


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
pub fn construct_event_object_graph(ocel: &OCEL) -> (DiGraph<String, ()>, HashMap<String, NodeIndex>) {
    #[cfg(feature = "progress")]
    println!("constructing Event Object Graph (EOG) ...");
    let mut id_to_node: HashMap<String, NodeIndex> = HashMap::new();
    //last event visited with object_id
    let mut last_event_index: HashMap<String, String> = HashMap::new(); // object_id, event_id
    let mut edges: Vec<(String, String)> = Vec::new(); //source, target

    let mut eog = DiGraph::<String, ()>::new();

    for i in 0..ocel.events.len() {
        //print %
        #[cfg(feature = "progress-percentage")]
        println!("EOG: {}%", (i as f32 / ocel.events.len() as f32 * 100.0));
        let event = &ocel.events[i];
        id_to_node.insert(event.id.clone(), eog.add_node(event.id.clone())); //add event_id as weight

        for obj in &event.relationships{
            //if object is in last_event_index, add edge
            if let Some(last_event_id) = last_event_index.get(&obj.object_id) {
                //add edge from last event to current event
                edges.push((last_event_id.clone(), event.id.clone()));
            }
            //update last_event_index with current event
            last_event_index.insert(obj.object_id.clone(), event.id.clone());

        }
    }

    //add edges to eog
    for (source, target) in edges{ //source/target: event_ids

        eog.add_edge(*id_to_node.get(&source).unwrap(), *id_to_node.get(&target).unwrap(), ());
    }
    (eog, id_to_node)

    
    //TODO: order of Events?
    //TODO: use somthing like: arr = ocel.to_numpy()


    /*
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

     */

}

fn calc_preset_recurv(eog: &DiGraph<String, ()>, event_id: String, id_to_node: &HashMap<String, NodeIndex>, presets: &mut HashMap<String, HashSet<String>>) -> HashSet<String> {

    if presets.contains_key(&event_id){
        return presets[&event_id].clone();
    }

    let predecessors: Vec<NodeIndex> = eog.neighbors_directed(*id_to_node.get(&event_id).unwrap(), rustworkx_core::petgraph::Direction::Incoming).collect();

    //if event has no predecessors, return empty vector
    if predecessors.is_empty(){
        presets.insert(event_id.clone(), HashSet::new());
        HashSet::new()
    }else{

        //add preset of each predecessor and predecessor itself to preset

        let mut preset: HashSet<String> = HashSet::new();
        for predecessor in predecessors {
            let predecessor_event_id = eog[predecessor].clone();
            //recursively get preset of predecessor
            let predecessor_preset = calc_preset_recurv(eog, predecessor_event_id.clone(), id_to_node, presets);
            preset.extend(predecessor_preset);
            preset.insert(predecessor_event_id);
        }
        presets.insert(event_id.clone(), preset.clone());
        preset
    }

}

//TODO: not fully tested + return event ID?
pub fn get_event_presets(ocel: &OCEL, id_to_event: &HashMap<String, OCELEvent>) -> HashMap<String, Vec<OCELEvent>> {
    //TODO: order important? => sort on time attribute????
    #[cfg(feature = "progress")]
    println!("constructing presets ...");

    //get preset => all events that are connected to event_id in the eog
    //=> get all predecessors of event_id

    //TODO: does function change ocel?
    let (eog, id_to_node) = construct_event_object_graph(ocel);
    //key: id; value: preset
    let mut presets: HashMap<String, HashSet<String>> = HashMap::new();
    
    /*
    let mut presets_: HashMap<String, Vec<OCELEvent>> = HashMap::new();
    //get presets OLD
    for node in eog.node_indices() {
        //print %
        //println!("Presets: {}%", (presets.len() as f32 / eog.node_count() as f32 * 100.0));


        let ancestors: Vec<NodeIndex> = ancestors(&eog, node).filter(|n| *n != node).collect();
        let event_id = eog[node].clone();
        //to get ancestors as event_ids and not NodeIndex
        let ancestors_str: Vec<String> = ancestors.iter().map(|n| eog[*n].clone()).collect();

        //get Event of event_ids
        let ancestors_events: Vec<OCELEvent> = ancestors_str.iter()
            .filter_map(|id| ocel.events.iter().find(|e| e.id == *id))
            .cloned()
            .collect();

        presets_.insert(event_id.clone(), ancestors_events);
    }
    return presets_;
    */
     
    
     
    
    for event_id in ocel.events.iter().map(|e| e.id.clone()) {

        #[cfg(feature = "progress-percentage")]
        println!("Presets: {}%", (presets.len() as f32 / eog.node_count() as f32 * 100.0));

        //if presets already contains event_id, continue
        if presets.contains_key(&event_id) {
            continue;
        }

        //else calc preset
        calc_preset_recurv(&eog, event_id.clone(), &id_to_node, &mut presets);

    }
    
    
    //TODO temp: convert do OCELEvent
    let mut event_presets: HashMap<String, Vec<OCELEvent>> = HashMap::new();
    for (event_id, preset) in presets {
        //get all events with event_id in ocel
        event_presets.insert(event_id, preset.iter().map(|event_id| id_to_event[event_id].clone()).collect());
    }
    event_presets
    
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
}

//TODO: not fully tested
//returns context and expanded bindings
pub fn _get_contexts_and_bindings(
    ocel: &OCEL, 
    id_to_event: &HashMap<String, OCELEvent>, 
    comp_presets: &Option<HashMap<String, Vec<OCELEvent>>>,
    all_object_types: &HashSet<String>,
    object_to_type: &HashMap<String, String>)
    -> (HashMap<String, HashMap<String, Counter<Vec<String>>>>,
        HashMap<String, Vec<(String, HashMap<String,HashSet<String>>)>>) {
    #[cfg(feature = "progress")]
    println!("constructing contexts and bindings...");
    
    let mut contexts: HashMap<String, HashMap<String, Counter<Vec<String>>>> = HashMap::new(); //event_id, HashMap [object_type, Counter( activities)]
    let mut bindings: HashMap<String, Vec<(String, HashMap<String,HashSet<String>>)>> = HashMap::new(); //map of binding sequences: event_id, Vec< activity: object_types: objects>

    let presets: HashMap<String, Vec<OCELEvent>> = comp_presets.clone().unwrap_or_else(|| get_event_presets(ocel, id_to_event)); //calculate presets if not alaready given

    for (event_id, preset) in presets {

        let mut context: HashMap<String, Counter<Vec<String>>> = HashMap::new();
        let mut binding_sequence: Vec<(String, HashMap<String, HashSet<String>>)> = Vec::new(); //activity, HashMap [obejct_type, Vec<Objects with this type>]
        

        //object_type, object_id
        let mut associated_objects: HashMap<String, HashSet<String>> = HashMap::new();
        //add current objects to associated_objects
        for rel in id_to_event.get(&event_id).unwrap().relationships.clone() {
            associated_objects.entry(object_to_type[&rel.object_id].clone())
                .or_insert_with(HashSet::new)
                .insert(rel.object_id.clone());
        }

        //get all the associated objects for preset
        for event in preset.clone(){
            let mut associated_objects_of_event: HashMap<String, HashSet<String>> = HashMap::new();
            event.relationships.iter()
                .map(|rel| rel.object_id.clone())
                .for_each(|object_id| {
                    let object_type = object_to_type.get(&object_id).unwrap();
                    associated_objects.entry(object_type.clone())
                        .or_insert_with(HashSet::new)
                        .insert(object_id.clone());
                    associated_objects_of_event.entry(object_type.clone())
                        .or_insert_with(HashSet::new)
                        .insert(object_id.clone());
                });

            //add object to binding_sequence
            binding_sequence.push((event.event_type.clone(), associated_objects_of_event));

        }
        //addisiotnaly push the current event to the binding_sequence for expanded binding sequence
        //binding_sequence.push((id_to_event.get(&event_id).unwrap().event_type.clone(), get_associated_objects(id_to_event.get(&event_id).unwrap(), &objects_to_objecttypes)));
        
        for object_type in all_object_types.clone(){
            
            let objects: Option<&HashSet<String>> = associated_objects.get(&object_type);
            
            if objects.is_none() {
                //add empty context for object_type if it does not exist
                context.entry(object_type.clone())
                    .or_insert_with(Counter::new).update(vec![vec![]]);
                continue;
            }
            
            for object in objects.unwrap() {
                //TODO: maybe more efficient if we calc the prefix for all objects at once
                //prefix all activities that are associated with this object
                let mut prefix: Vec<String> = preset.iter()
                    .filter(|e| e.relationships.iter().any(|rel| rel.object_id == *object))
                    .map(|e| e.event_type.clone())
                    .collect();
                
                prefix.sort(); //sort prefix to ensure consistent order TODO: is multiset + hasher more efficient?
              
                context.entry(object_type.clone())
                    .or_insert_with(Counter::new)
                    .update(vec![prefix.clone()]);
                //println!("[DEBUG] Context for event {}: {:?}", event_id, context.get(&object_type.clone()));
            }
        }
        //add context to contexts
        contexts.insert(event_id.clone(), context);
        //add binding_sequence to bindings
        bindings.insert(event_id.clone(), binding_sequence);
    }
    //println!("[DEBUG] contexts: {:#?}", contexts);
    //TODO: some inconsistencies with order of activities in context
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

        if let Some(counter) = context.get(key) {

            let mut sorted_counter_keys: Vec<(&Vec<String>, &usize)> = counter.iter().collect();
            sorted_counter_keys.sort_by(|(a, _), (b, _)| a.cmp(b)); // sorting

            for (vec, count) in sorted_counter_keys {
                let mut sorted_vec = vec.clone();
                sorted_vec.sort(); //sort inner vec
                sorted_vec.hash(&mut hasher);
                count.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}



//TODO: empty contexts problem
pub fn get_enabled_log_activities(
    contexts: &HashMap<String, HashMap<String, Counter<Vec<String>>>>,
    id_to_event: &HashMap<String, OCELEvent>, ) 
    -> (HashMap<String, HashSet<String>>, HashMap<u64, HashSet<String>>) {
    #[cfg(feature = "progress")]
    println!("getting enabled log activities ...");

    let mut seen_contexts: HashMap<u64, HashSet<String>> = HashMap::new(); //hashed context, event_id
    let mut context_event_map: HashMap<u64, HashSet<String>> = HashMap::new(); //hashed context, event_ids
    //create context map
    for (event_id, context) in contexts{
        let hashed_context = hash_context(context);

        //if context is already seen, add event_id to seen_contexts
        if !seen_contexts.contains_key(&hashed_context) {
            seen_contexts.insert(hashed_context, HashSet::from_iter([event_id.clone()]));
            context_event_map.insert(hashed_context, HashSet::from_iter([event_id.clone()]));
        } else {
            seen_contexts.get_mut(&hashed_context).unwrap().insert(event_id.clone());
            context_event_map.get_mut(&hashed_context).unwrap().insert(event_id.clone());
        }
    }
    let mut enabled_log_activities: HashMap<String, HashSet<String>> = HashMap::new(); //event_id, activities
    for (context, event_ids) in seen_contexts {

        let en_activities: HashSet<String> = event_ids.iter()
            .map(|event_id| id_to_event.get(event_id).unwrap().event_type.clone()).collect();

        for event_id in event_ids {
            enabled_log_activities.entry(event_id.clone())
                .or_default()
                .extend(en_activities.clone());
        }
    }

    (enabled_log_activities, context_event_map)
}

