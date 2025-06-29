extern crate process_mining as pm;

use std::collections::{HashMap, HashSet};
use bimap::BiMap;
use uuid::Uuid;
use crate::utils::*;

use petgraph::dot::{Dot, Config};
use petgraph::Graph;
use std::fs::File;
use std::io::Write;
use petgraph::graph::DiGraph;
use petgraph::visit::NodeRef;

//OCPN
#[derive(Clone)]
pub struct OCPN {

    pub places: HashMap<String, Place>, // <place id, Place> TODO: maybe add object types?

    pub transitions: HashMap<String, Transition>, //<transition_ids / activities, Transition>
    pub silent_transtions: HashMap<String, Transition>, //transition ids, Transition

    pub arcs: Vec<Arc>,

    pub initial_marking: Option<Marking>,
    pub final_marking: Option<Vec<Marking>>,

    pub object_to_type: HashMap<String, String>, //object id, object type
    pub activity_to_transitions: HashMap<String, Vec<String>>, //map from activity to all tranisitions with that activity for faster access

}


impl OCPN {

    //------------------ CONSTRUCTORS -------------------------------

    //add arc to the OCPN: source: place id, target: transition id, object_type: object type
    ///if successful return (source_id, target_id)
    pub fn add_arc_to_transition(&mut self, source_id: Option<String>, target_id: Option<String>, object_type: String, activity: Option<String>) -> Result<(String, String), String> {

        let mut t_id = "".to_string();

        if target_id.is_none() || (!self.transitions.contains_key(&target_id.clone().unwrap())&& !self.silent_transtions.contains_key(&target_id.clone().unwrap())) {
            //target transition does not exist yet
            if let Some(act) = activity{
                t_id = self.add_transition(act, target_id.clone(), Some(false))?;
            }else{
                //when transition does not exists and no activity is given, no new transition can be created
                return Err("No transition found and can't create new transition because activity is missing".to_string())
            }

        } else if self.transitions.contains_key(&target_id.clone().unwrap()) || self.silent_transtions.contains_key(&target_id.clone().unwrap()) {
            t_id = target_id.unwrap();
        } else {
            //when transition does not exists and no activity is given, no new transition can be created
            return Err("Error".to_string())
        }


        //source place
        let mut p_id = "".to_string();
        if source_id.is_none() || !self.places.contains_key(&source_id.clone().unwrap()) {
            //target place does not exist yet
            p_id = self.add_place(source_id, object_type.clone())?;
        }else{
            p_id = source_id.unwrap();
        }

        self.transitions.get_mut(&t_id).unwrap().input_places.push((p_id.clone(), object_type.clone()));
        self.places.get_mut(&p_id).unwrap().output_transitions.push((t_id.clone(), object_type.clone()));

        let arc = Arc {
            source: p_id.clone(),
            target: t_id.clone(),
            object_type,
        };
        self.arcs.push(arc);
        Ok((p_id, t_id))
    }
    //add arc to the OCPN: source: transition id, target: place id, object_type: object type
    ///if successful return (source_id, target_id)
    pub fn add_arc_from_transition(&mut self, source_id: Option<String>, target_id: Option<String>, object_type: String, activity: Option<String>) -> Result<(String, String), String> {

        let mut t_id = "".to_string();

        if source_id.is_none() || (!self.transitions.contains_key(&source_id.clone().unwrap()) && !self.silent_transtions.contains_key(&target_id.clone().unwrap())) {
            //source transition does not exist yet
            if let Some(act) = activity{
                t_id = self.add_transition(act, source_id, Some(false))?;
            }else{
                //when transition does not exists and no activity is given, no new transition can be created
                return Err("No transition found and can't create new transition because activity is missing".to_string())
            }

        } else if self.transitions.contains_key(&source_id.clone().unwrap()) || self.silent_transtions.contains_key(&source_id.clone().unwrap()) {
            t_id = source_id.unwrap();
        } else {
            //when transition does not exists and no activity is given, no new transition can be created
            return Err("Error".to_string())
        }


        //target place
        let mut p_id = "".to_string();
        if target_id.is_none() || !self.places.contains_key(&target_id.clone().unwrap()) {
            //target place does not exist yet
            p_id = self.add_place(target_id, object_type.clone())?;
        }else{
            p_id = target_id.unwrap();
        }

        self.transitions.get_mut(&t_id).unwrap().output_places.push((p_id.clone(), object_type.clone()));
        self.places.get_mut(&p_id).unwrap().input_transitions.push((t_id.clone(), object_type.clone()));

        let arc = Arc {
            source: t_id.clone(),
            target: p_id.clone(),
            object_type,
        };
        self.arcs.push(arc);
        Ok((t_id, p_id))
    }
    ///if successful return (source_id, target_id)
    pub fn add_arc(&mut self, source_id: Option<String>, target_id: Option<String>, object_type: String, activity:Option<String>) -> Result<(String, String), String> {

        /*
         cases: 
           source      target
           
           None         None -> err
           None         place -> from trans
           None         trans -> to trans
           place        None  -> to trans
           place        place -> err
           place        trans -> to trans
           trans        None  -> from trans
           trans        place -> from trans
           trans        trans -> err
         */
        
        if source_id.is_none(){
            if target_id.is_none(){
                Err("Source and Target can't both be none!".to_string())
            }
            else if self.places.contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else{
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            
        }
        else if self.places.contains_key(&source_id.clone().unwrap()){
            if target_id.is_none(){
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else if self.places.contains_key(&target_id.clone().unwrap()){
                Err("Place to Place is not allowed!".to_string())
            }
            else{
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            
        }
        else{//source must be transition
            if target_id.is_none(){
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else if self.places.contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else{
                Err("Transition to Transition is not allowed!".to_string().to_string())
            }
        }

        /*
        //check if source is a place or a transition
        if self.places.contains_key(&source_id) && !self.places.contains_key(&target_id) {

            //target is either a transition, silent transition or not defined
            self.add_arc_to_transition(source_id, target_id, object_type, activity);

        } else if (self.transitions.contains_key(&source_id) || self.silent_transtions.contains_key(&source_id)) &&
            (!self.transitions.contains_key(&target_id) || !self.silent_transtions.contains_key(&target_id)) {
            
            //source is either a transition, silent transition or not defined
            self.add_arc_from_transition(source, target, object_type);
        } else {
            panic!("Source and target can't be both places or both transitions. Source: {}, Target: {}. Or both places are unknown", source, target);
        }

         */

    }
    pub fn add_transition(&mut self, activity_name: String, transition_id: Option<String>, is_silent_transition: Option<bool>) -> Result<String, String> {

        //check if it is silent transition
        if is_silent_transition == Some(true) {
            self.add_silent_transition(activity_name.clone(), transition_id.clone());
        }

        //default id
        let mut id:String = transition_id.clone().unwrap_or(
            String::from("t".to_string() + &self.transitions.len().to_string()));

        //check if id already exists
        if self.transitions.contains_key(&id) {Err(format!("Transition \"{}\" already exists", id))? }

        //add a new transition to the OCPN
        let transition = Transition {
            input_places : vec![], //no input places by default
            output_places: vec![], //no output places by default
            activity: activity_name.clone(),
        };

        //default is not silent transition
        self.transitions.insert(id.clone(), transition);
        self.activity_to_transitions.entry(activity_name.clone()).or_insert(vec![]).push(id.clone());

        //add to activity to transition
        self.activity_to_transitions.entry(activity_name.clone()).or_insert(vec![]).push(id.clone());
        Ok(id)
    }

    pub fn add_silent_transition(&mut self, activity_name: String, transition_id: Option<String>) -> Result<String, String> {
        //add a new silent transition to the OCPN

        //default id
        let mut id:String = transition_id.clone().unwrap_or(
            String::from("st".to_string() + &self.silent_transtions.len().to_string()));
        if self.silent_transtions.contains_key(&id) { Err(format!("Silent transition \"{}\" already exists", id))? }

        //add a new transition to the OCPN
        let transition = Transition {
            input_places : vec![], //no input places by default
            output_places: vec![], //no output places by default
            activity: activity_name.clone(),
        };

        self.silent_transtions.insert(id.clone(), transition);
        self.activity_to_transitions.entry(activity_name.clone()).or_insert(vec![]).push(id.clone());
        Ok(id)
    }

    pub fn add_place(&mut self, place_id: Option<String>, object_type: String) -> Result<String, String> {
        //add a new place to the OCPN

        let id:String = place_id.clone().unwrap_or(
            String::from("pl".to_string() + &self.places.len().to_string()));
        if self.places.contains_key(&id) { Err(format!("Place \"{}\" already exists", id))? }


        let place = Place {
            input_transitions: vec![], //no input transitions by default
            output_transitions: vec![], //no output transitions by default
            object_type: object_type.clone(), //set object type of the place
        };
        self.places.insert(id.clone(), place);
        Ok(id)
    }

    pub fn new(object_to_type: HashMap<String, String>) -> Self {
        //create a new OCPN
        OCPN {
            places: HashMap::new(),
            transitions: HashMap::new(),
            silent_transtions: HashMap::new(),
            arcs: vec![],
            initial_marking: None,
            final_marking: None,
            object_to_type,
            activity_to_transitions: HashMap::new(),
        }
    }



    //------------------ EXPORTS --------------------------------

    pub fn export_to_petgraph(&self) -> DiGraph<String, String> {
        //construct graph
        let mut graph = DiGraph::<String, String>::new();

        // construct graph from OCPN
        let mut nodes: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
        for place in self.places.iter() {
            nodes.insert(place.0.clone(), graph.add_node(place.0.clone()));
        }
        for transition in self.transitions.iter() {
            nodes.insert(transition.0.clone(), graph.add_node(transition.0.clone()
                + ":  " + &self.transitions.get(&transition.0.clone()).unwrap().activity));
        }
        for silent_transition in self.silent_transtions.iter() {
            nodes.insert(silent_transition.0.clone(), graph.add_node(silent_transition.0.clone()));
        }
        for arc in &self.arcs {
            let source = nodes.get(&arc.source).expect("Source node not found");
            let target = nodes.get(&arc.target).expect("Target node not found");
            graph.add_edge(*source, *target, arc.object_type.clone());
        }

        graph
    }
    pub fn export_dot_to_file(&self, path:String){

        let graph = self.export_to_petgraph().clone();

        //visualize graph (export)
        let dot = Dot::with_config(&graph, &[]);

        //save eog to vis.dot
        let mut file = File::create(path.clone()).expect("error");
        write!(file, "{:?}", dot).expect("error");

        println!("graph saved in {:?}; run with: dot -Tpng {} -o [file name].png", file, path);

    }



    //---------------- GETTERS --------------------------------

    pub fn get_enabled_transitions_with_breakcondition(&self, marking: &Marking, binding_sequence: Option<&HashMap<String, Vec<Vec<String>>>>) -> Vec<(String, Vec<String>)> { //return: transition_id, [object ids that are involved]
        //ASSUMPTION: binding inputs matches the input places of the transitions !!!! TODO
        //get enabled transitions from the marking
        //if binding_sequence != None: return first found transition that is also in binding_sequence

        let mut enabled_transitions_with_obj_ids : Vec<(String, Vec<String>)> = vec![]; //transition_id, [object ids that are involved]

        //get all transtions that are associated with the all places in the marking
        //TODO: faster to directly calc or filter with assitiated transitions
        let assotiated_transitions: HashSet<String> = self.get_associated_transitions_of_marking(marking, false);

        //filter transitions that have all input places in the marking
        for transition_id in assotiated_transitions {
            //get the transition
            if let Some(transition) = self.transitions.get(&transition_id) {
                //get the required inputs (if one imput is needed : // (place_id, object_type), count of objects needed
                let required_inputs = self.get_required_input_places_and_obj_types(&transition.clone());

                let mut involved_objects: Vec<String> = vec![]; //objects that are involved in this transition

                //check if all required input places are in the marking
                let mut is_enabled = true;
                for ((place_id, object_type), count) in required_inputs {

                    match marking.get(&place_id) {
                        Some(objects) => {
                            // count how many objects with object_type are in the marking for this place

                            let matching_count = objects.iter()
                                .filter(|obj_id| {
                                    let matches = *self.object_to_type.get(*obj_id).unwrap() == object_type;
                                    if matches {
                                        involved_objects.push(obj_id.clone().to_string()); // Objekt pushen
                                    }
                                    matches // Rückgabe für den Filter
                                })
                                .count();

                            if matching_count < count {
                                is_enabled = false;
                                break;
                            }
                        }
                        None => {
                            is_enabled = false;
                            break;
                        }
                    }
                }
                //this transition is enabled
                if is_enabled {

                    enabled_transitions_with_obj_ids.push((transition_id.clone(), involved_objects.clone()));
                    //if binding_sequence is provided, check if this transition is in the binding_sequence
                    if let Some(binding_seq) = binding_sequence {
                        if binding_seq.contains_key(&transition_id) {
                            //if it is, return it immediately
                            return vec![(transition_id, involved_objects.clone())];
                        }
                    }
                }
            }

        }

        //if breakting condition is provided (binding_sequence is Some), and it has not returned a transition yet, return empty vector
        if let Some(binding_seq) = binding_sequence {
            return vec![];
        }
        enabled_transitions_with_obj_ids
    }

    pub fn get_enabled_transitions_from_marking(& self, marking: &Marking) -> Vec<String> {
        //get enabled transitions from the marking with no break condition
        self.get_enabled_transitions_with_breakcondition(marking, None).iter().map(|transition| transition.0.clone()).collect()
    }

    pub fn get_associated_transitions_of_marking(&self, marking: &Marking, silent_transition: bool) -> HashSet<String> {
        //get all transtions that are associated with the all places in the marking
        //TODO: faster to directly calc or filter with assitiated transitions

        let associated_transitions: HashSet<String> = marking.keys()
            .flat_map(|place_id| {
                self.places.get(place_id)
                    .map(|p| {
                        p.output_transitions.iter()
                            //only include transitions that are either silent or not silent, depending on the parameter
                            .filter(|(tid, _)| (silent_transition && self.silent_transtions.contains_key(tid)) || (!silent_transition && self.transitions.contains_key(tid)))
                            .map(|(tid, _)| tid.clone())
                    })
                    .into_iter()
                    .flatten()
            })
            .collect();


        associated_transitions
    }

    pub fn get_required_input_places_and_obj_types(&self, transition: &Transition) -> HashMap<(String, String), usize> { //(place_id, object_type), count of objects needed
        //(place_id, object_type), count of objects needed
        let mut required: HashMap<(String, String), usize> = HashMap::new();
        for (place_id, object_type) in transition.input_places.clone() {
            *required.entry((place_id.clone(), object_type.clone())).or_insert(0) += 1;
        }
        required
    }

    pub fn remove_next_binding(&self, binding_sequence: &mut HashMap<String, Vec<Vec<String>>>, marking: &Marking) -> Option<(String, Vec<String>)> {//event_type, [object_ids (for first event with this event_type)]>
        //Binding Sequence: <event_type, [[object_ids (for first event with this event_type)], [object_ids (for second event with this event_type), ...]]>

        //get first enabled transition from the marking
        let enabled_transition = self.get_enabled_transitions_with_breakcondition(&marking, Some(binding_sequence));
        if enabled_transition.is_empty() {
            //no enabled transition found
            return None;
        }


        //remove and return fist binding that fits
        for (transition_id, object_ids) in enabled_transition.iter() {
            //get first binding that satisfies: transition_id and object_ids
            let big_binding = binding_sequence.get_mut(transition_id).unwrap();


            //iterate over each (real) binding in big binding
            for index in (0..big_binding.len()).rev() {  // Iterate backwars because swap_remove
                if is_superset(&object_ids, &big_binding[index]) {
                    let result = (transition_id.clone(), big_binding.swap_remove(index));
                    //if binding is empty, remove it from the sequence
                    if big_binding.is_empty() {
                        binding_sequence.remove(transition_id);
                    }
                    return Some(result);
                }
            }

        }
        None
    }

    pub fn get_enabled_silent_transition_bindings(&self, marking: &Marking) -> Vec<(String, Vec<String>)> { //return transition_id, [object ids that are involved]
        
        //what is better check all silent transitions or check all places in Marking? ==> compare size of both
        
        let mut enabled_silent_transition_bindings: Vec<(String, Vec<String>)> = vec![]; //result
        
        let mut assotiated_silent_transitions: Vec<(String, String)> = vec![]; //transition_id, object type
        
        if true || marking.len() <= self.silent_transtions.len() {
            //iterate over marking => is smaller
            for (place_id, objects) in marking.iter() {
                //check if this place has a silent transition
                assotiated_silent_transitions.extend(self.get_associated_silent_transition_bindings_from_marking(place_id.clone()));
            }
            
        }else {
            //iterate over silent transitions => is smaller TODO
        }
        
        //filter associated silent transitions that are enabled
        for (transition_id, _) in assotiated_silent_transitions {
            
            //check if silent transition is enabled: are all inputs satisfied?
            let (is_enabled, involved_objects) = self.is_transition_enabled(marking, transition_id.clone());
            if is_enabled {
                enabled_silent_transition_bindings.push((transition_id, involved_objects.unwrap()));
            }
        }
        enabled_silent_transition_bindings
    }
    
    //TODO: Merge with get_associated_transitions_of_marking; what is the difference??
    fn get_associated_silent_transition_bindings_from_marking(&self, place_id:String) -> Vec<(String, String)> { //return transition_id, object type
        //get associated silent transitions from the marking
        let mut associated_silent_transitions_with_obj_type: Vec<(String, String)> = vec![]; //transition_id, object type

        for (transition_id, object_type) in self.places.get(place_id.as_str()).unwrap().output_transitions.iter() {
            
            if !self.silent_transtions.contains_key(transition_id.as_str()) { continue; } //not a silent transition
            
            associated_silent_transitions_with_obj_type.push((transition_id.clone(), object_type.clone())); 
        }
        associated_silent_transitions_with_obj_type
    }


    //---------------- EXECUTION --------------------------------

    pub fn execute_binding(&self, binding: &mut (String, Vec<String>), marking: &Marking) -> Marking{ ///binding: (transition_id, [object_ids])

        let mut res_marking = marking.clone(); //clone marking to modify it
        let mut temp_binding = binding.clone(); //clone binding to modify it

        //remove objects from input places
        for (place_id,_) in self.transitions.get(&binding.0).unwrap_or_else(||self.silent_transtions.get(&binding.0).unwrap()).input_places.clone() {
            //remove object from input place
            //res_marking.get_mut(&place_id).unwrap().remove(&temp_binding.1.pop().unwrap());
            res_marking.get_mut(&place_id).unwrap().remove(&pop_object_from_binding(&mut temp_binding, &self.places.get(&place_id).unwrap().object_type, &self.object_to_type.clone()).unwrap());


            //if place is empty, remove it from the marking
            if res_marking.get(&place_id).unwrap().is_empty() {
                res_marking.remove(&place_id);
            }
        }
//TODO: too many unnesessary iterations => maybeo only iterate over object types
        //add objects from output places
        for (place_id, _) in self.transitions.get(&binding.0).unwrap_or_else(||self.silent_transtions.get(&binding.0).unwrap()).output_places.clone() {
            res_marking.entry(place_id.clone()).or_insert_with(HashSet::new)
                .extend(binding.1.iter().filter(|&object_id| *self.object_to_type.get(object_id).unwrap() == self.places.get(&place_id).unwrap().object_type).cloned().collect::<HashSet<String>>()); //add all objects of the binding that match the object type of the place
        }

        res_marking
    }



    //------------------ CHECKS --------------------------------

    //TODO: function code is based on get_enabled_transitions_with_breakcondition
    pub fn is_transition_enabled(&self, marking: &Marking, transition_id: String) -> (bool, Option<Vec<String>>) {

        let required_inputs = self.get_required_input_places_and_obj_types(&self.transitions.get(&transition_id).or_else(|| self.silent_transtions.get(&transition_id)).unwrap().clone());

        let mut involved_objects: Vec<String> = vec![]; //objects that are involved in this transition

        for ((place_id, object_type), count) in required_inputs {

            match marking.get(&place_id) {
                Some(objects) => {
                    // count how many objects with object_type are in the marking for this place

                    let matching_count = objects.iter()
                        .filter(|obj_id| {
                            let matches = *self.object_to_type.get(*obj_id).unwrap() == object_type;
                            if matches {
                                involved_objects.push(obj_id.clone().to_string()); // Objekt pushen
                            }
                            matches // Rückgabe für den Filter
                        })
                        .count();

                    if matching_count < count {
                        return (false, None);
                    }
                }
                None => {
                    return (false, None);
                }
            }
        }
        (true, Some(involved_objects)) //transition is enabled, return involved objects
    }

    fn all_transitions_contains_key(&self, key: &String) -> bool {
        self.transitions.contains_key(key) || self.silent_transtions.contains_key(key)
    }

}

#[derive(Clone)]
pub struct Place{
    pub input_transitions: Vec<(String, String)>, //transition ids, object type
    pub output_transitions: Vec<(String, String)>, //transition ids, object type
    pub object_type: String //object type of the place
}
#[derive(Clone)]
pub struct Transition {
    pub input_places: Vec<(String, String)>, //place ids, object type
    pub output_places: Vec<(String, String)>, //place ids, object type

    pub activity: String, //activity
}
#[derive(Clone)]
pub struct Arc {
    pub source: String, //place id/ transition id
    pub target: String, //place id/ transition id
    pub object_type: String, //object type
}


/// Marking: <place id, [object ids, -DELETED: Object type]>
pub type Marking = HashMap<String, HashSet<String>>; //place id, [object ids, -DELETED: Object type]




