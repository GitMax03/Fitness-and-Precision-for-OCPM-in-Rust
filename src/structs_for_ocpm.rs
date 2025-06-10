extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::utils::*;




 
//OCPN
pub struct OCPN {
    // <place id, Place> TODO: maybe add object types?
    pub places: HashMap<String, Place>,
    //<transition_ids / activities, Transition>
    pub transitions: HashMap<String, Transition>,
    pub silent_transtions: HashMap<String, Transition>, //transition ids, Transition

    pub arcs: Vec<Arc>,


    pub initial_marking: Option<Marking>,
    pub final_marking: Option<Vec<Marking>>,



}

impl OCPN {

    //add arc to the OCPN: source: place id, target: transition id, object_type: object type
    pub fn add_arc_to_transition(&mut self, source: String, target: String, object_type: String) {

        //add inputplace to the transition
        if let Some(transition) = self.transitions.get_mut(&target) {
            //if target transition exists, add the input place
            transition.input_places.push((source.clone(), object_type.clone()));
        } else {
            //if no transition exists, create a new one
            self.transitions.insert(target.clone(), Transition {
                input_places: vec![(source.clone(), object_type.clone())],
                output_places: vec![],
            });
        }
        //add output place to the source place
        if let Some(place) = self.places.get_mut(&source) {
            //if source place exists, add the output transition
            place.output_transitions.push((target.clone(), object_type.clone()));
        } else {
            //if no place exists, create a new one
            self.places.insert(source.clone(), Place {
                input_transitions: vec![],
                output_transitions: vec![(target.clone(), object_type.clone())],
            });
        }



        let arc = Arc {
            source,
            target,
            object_type,
        };
        self.arcs.push(arc);
    }

    //add arc to the OCPN: source: transition id, target: place id, object_type: object type
    pub fn add_arc_from_transition(&mut self, source: String, target: String, object_type: String) {

        //add output place to the transition
        if let Some(transition) = self.transitions.get_mut(&source) {
            //if source transition exists, add the output place
            transition.output_places.push((target.clone(), object_type.clone()));
        } else {
            //if no transition exists, create a new one
            self.transitions.insert(source.clone(), Transition {
                input_places: vec![],
                output_places: vec![(target.clone(), object_type.clone())],
            });
        }
        //add input place to the target place
        if let Some(place) = self.places.get_mut(&target) {
            //if target place exists, add the input transition
            place.input_transitions.push((source.clone(), object_type.clone()));
        } else {
            //if no place exists, create a new one
            self.places.insert(target.clone(), Place {
                input_transitions: vec![(source.clone(), object_type.clone())],
                output_transitions: vec![],
            });
        }


        let arc = Arc {
            source,
            target,
            object_type,
        };
        self.arcs.push(arc);
    }


    //return: (activity(transtion id), binding), (transition id, input places)
    pub fn find_first_enabled_binding_and_transition(&self, marking: &Marking, binding_sequence: &HashMap<String, Vec<Vec<String>>>) ->  Option<((String, Vec<String>), (String, Vec<String>))> {

        //TODO amrking not used??????


        //binding seq: <event_type, [[object_ids (for first event with this event_type)], [object_ids (for second event with this event_type), ...]]>
        //check each binding in the sequence for possibility to be enabled
        for binding in binding_sequence {

            //get input places for this binding: place ids
            let required_input_places: Vec<String> = self.transitions.get(binding.0).unwrap()//get all transitions of binding
                .input_places.iter().map(|(place_id,_)| place_id.clone()).collect(); //get ids of all input places

            let mut marking_temp = marking.clone();
            //check if each required input place is in the marking
            if required_input_places.iter().all(|place_id| {
                if let Some(objects) = marking_temp.remove(place_id) {
                    //check if object is correct
                    //TODO: One object can't be in one marking multiple times : true???



                }else{
                    return false;
                }
            }) {
                
            }


            for required_objects in binding.1 {
                //check if all required objects are in the marking
                if is_superset(input_places.clone(), required_objects) {
                    //all required objects are in the marking
                    return Option::from(((binding.0.clone(), required_objects.clone()), binding.0.clone()));
                }
            }
        }
        //no binding found
        None
    }


    pub fn get_next_marking(&self, marking: &Marking, firing_transition_id: String, input_places: Vec<String>) -> Option<Marking> {

        for (place_id, objects) in marking {
            //if place is input place for the firing transition, remove objects from it
            if input_places.contains(place_id) {
                //remove all objects from this place
                if let Some(transition) = self.transitions.get(&firing_transition_id) {
                    //check if this transition has output places
                    for (output_place_id, _) in &transition.output_places {
                        //add objects to the output place
                        let mut new_marking = marking.clone();
                        new_marking.entry(output_place_id.clone()).or_insert_with(Vec::new).extend(objects.clone());
                        return Some(new_marking);
                    }
                }
            }
        }



        Noen
    }
}


pub struct Place{
    pub input_transitions: Vec<(String, String)>, //transition ids, object type
    pub output_transitions: Vec<(String, String)>, //transition ids, object type
}

pub struct Transition {
    pub input_places: Vec<(String, String)>, //place ids, object type
    pub output_places: Vec<(String, String)>, //place ids, object type
}

pub struct Arc {
    pub source: String, //place id/ transition id
    pub target: String, //place id/ transition id
    pub object_type: String, //object type
}



pub type Marking = HashMap<String, HashSet<String>>; //place id, [object ids, -DELETED: Object type]



 