extern crate process_mining as pm;

use std::collections::HashMap;
use rustworkx_core::petgraph::prelude::*;
use rustworkx_core::traversal::ancestors;


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
pub fn contruct_event_object_graph(ocel: &pm::OCEL) -> DiGraph<String, ()> {
    
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
        //TODO: is OCEL Realtionship omap?
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

            //if intercection of objects is not empty => add edge
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

//TODO: not fully tested
pub fn get_event_presets(ocel: &pm::OCEL) -> HashMap<String, Vec<String>> {
    //get preset => all events that are connected to event_id in the eog
    //=> get all predecessors of event_id

    //TODO: does function change ocel?
    let eog = contruct_event_object_graph(ocel);
    //key: id; value: preset
    let mut presets: HashMap<String, Vec<String>> = HashMap::new();

    
    /*
    ancestors 
    - maybe calc in own code if no good lib available
    - maybe more efficient to use own code
    
    - use hash map to store already calculated presets
    - use petgraph::visit::Dfs???
    - reverse edges, check whcih node is reachable for each x 
    - get x from reversed event_id list
    - calc preset recursively and store in hash map
     */

  
    //get presets
    for node in eog.node_indices() {
        let ancestors: Vec<NodeIndex> = ancestors(&eog, node).filter(|n| *n != node).collect();
        let event_id = eog[node].clone();
        //to get ancestors as event_ids and not NodeIndex
        let ancestors_str: Vec<String> = ancestors.iter().map(|n| eog[*n].clone()).collect();
        presets.insert(event_id.clone(), ancestors_str);
    }
    
    presets
}

