use std::collections::BTreeMap;
use std::hash::Hasher;
use std::hash::Hash;
extern crate process_mining as pm;

use std::hash::DefaultHasher;
use std::collections::{HashMap, HashSet};
use bimap::BiMap;
use uuid::Uuid;
use crate::utils::*;

use petgraph::dot::{Dot, Config};
use petgraph::Graph;
use std::fs::File;
use std::io::Write;
use std::ops::Index;
use petgraph::graph::DiGraph;
use petgraph::visit::NodeRef;
use rayon::in_place_scope;
use serde_json::json;

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
            p_id = self.add_place(source_id, object_type.clone(), false)?; //default is_initial is false
        }else{
            p_id = source_id.unwrap();
        }

        self.transitions.get_mut(&t_id).or(self.silent_transtions.get_mut(&t_id))
            .unwrap().input_places.push((p_id.clone(), object_type.clone()));
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

        if source_id.is_none() || (!self.transitions.contains_key(&source_id.clone().unwrap()) && !self.silent_transtions.contains_key(&source_id.clone().unwrap())) {
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
            p_id = self.add_place(target_id, object_type.clone(), false)?; //default is_initial is false
        }else{
            p_id = target_id.unwrap();
        }

        self.transitions.get_mut(&t_id).or(self.silent_transtions.get_mut(&t_id)).unwrap().output_places.push((p_id.clone(), object_type.clone()));
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
        
        let source_none = source_id.is_none();
        let target_none = target_id.is_none();
        
        if !source_none && self.places.contains_key(&source_id.clone().unwrap()){
            if !target_none && self.places.contains_key(&target_id.clone().unwrap()){
                Err("Place to Place is not allowed!".to_string())
            }
            else if !target_none && self.all_transitions_contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }else{
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            
        }
        else if !source_none && self.all_transitions_contains_key(&source_id.clone().unwrap()){
            
            if !target_none && self.places.contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else if !target_none && self.all_transitions_contains_key(&target_id.clone().unwrap()){
                Err("Transition to Transition is not allowed!".to_string().to_string())
            }else{
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
        }else{ //soruce id is either none or not contained in place or transitions
            
            if !target_none && self.places.contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_from_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }
            else if !target_none && self.all_transitions_contains_key(&target_id.clone().unwrap()){
                Ok(self.add_arc_to_transition(source_id, target_id, object_type, activity).expect("Error with adding arc"))
            }else{
                Err("Source and Target can't both be none!".to_string())
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
            return Ok(self.add_silent_transition(activity_name.clone(), transition_id.clone()).unwrap());
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

    pub fn add_place(&mut self, place_id: Option<String>, object_type: String, is_initial: bool) -> Result<String, String> {
        //add a new place to the OCPN

        let id:String = place_id.clone().unwrap_or(
            String::from("pl".to_string() + &self.places.len().to_string()));
        if self.places.contains_key(&id) { Err(format!("Place \"{}\" already exists", id))? }


        let place = Place {
            input_transitions: vec![], //no input transitions by default
            output_transitions: vec![], //no output transitions by default
            object_type: object_type.clone(), //set object type of the place
            is_initial, //set initial place
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


    //------------------ IMPORTS --------------------------------

    


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

        let colors: Vec<(String, String)> = vec![
            ("ff0000".to_string(), "ff6666".to_string()), //red
            ("00ff00".to_string(), "66ff66".to_string()),
            ("0000ff".to_string(), "6666ff".to_string()), //Blue
            ("ffff00".to_string(), "ffff66".to_string()),
            ("ff00ff".to_string(), "ff66ff".to_string()),
            ("00ffff".to_string(), "66ffff".to_string()),
        ];
        let mut all_object_types: HashMap<String, (String, String)> = HashMap::new();
        let mut c_index = 0; //color index
        for object_type in self.object_to_type.values() {
            if all_object_types.contains_key(object_type) { continue; }
            all_object_types.insert(object_type.clone(), (colors[c_index % colors.len()].0.clone(), colors[c_index % colors.len()].1.clone()));
            c_index += 1;
        }
        
        
        
        //visualize graph (export)
        //let dot = Dot::with_config(&graph, &[]);
        let dot = Dot::with_attr_getters(
            &graph,
            &[],

            // edge attributes
            &|_graph, edge_ref| {
                //get edge attributes
                let mut x = String::new();
                let edge_weight = edge_ref.weight();
                if let label = edge_weight {
                    if label.starts_with("offer") {
                        x = format!("color=\"#{}\", style=filled", "0000ff");
                    } else if label.starts_with("application") {
                        x = format!("color=\"#{}\", style=filled", "ff0000");
                    }
                }
                x
            },

            // Node attributes
            &|_graph, node_ref| {
                
                let node_label = node_ref.1.clone();
                let mut x:String = "".to_string();

                if node_label.starts_with("offer") {
                    x = format!("color = \"#{}\", style =filled", "6666ff");
                }else if node_label.starts_with("application") {
                    x = format!("color=\"#{}\", style=filled", "ff6666");
                }else if node_label.contains(":") {
                    x = format!("shape = box; style = filled; color = gray");
                }else {
                    x = format!(" shape = box; style = filled; color = \"#575454\"");
                }
                
                x
            },
        );

        //save eog to vis.dot
        let mut file = File::create(path.clone()).expect("error");
        write!(file, "{:?}", dot).expect("error");

        println!("graph saved in {:?}; run with: dot -Tpng {} -o [file name].png", file, path);

    }



    //---------------- GETTERS --------------------------------
    ///return: transition_id, [object ids that are involved]
    pub fn get_enabled_transitions_with_breakcondition(&self, marking: &Marking, breaking_transitions: Option<&HashSet<String>>) -> Vec<(String, Vec<String>)> {

        //ASSUMPTION: binding inputs matches the input places of the transitions !!!! TODO
        //get enabled transitions from the marking
        //if binding_sequence != None: return first found transition that is also in binding_sequence

        let mut enabled_transitions_with_obj_ids : Vec<(String, Vec<String>)> = vec![]; //transition_id, [object ids that are involved]

        //get all transtions that are associated with the all places in the marking
        //TODO: faster to directly calc or filter with assitiated transitions?
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
                                        involved_objects.push(obj_id.clone().to_string());
                                    }
                                    matches
                                })
                                .count();

                            if matching_count < count { //not enough objects of the required type in the marking
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
                    if let Some(binding_seq) = breaking_transitions {
                        if binding_seq.contains(&self.transitions[&transition_id].activity) {
                            //if it is, return it immediately
                            return vec![(transition_id, involved_objects.clone())];
                        }
                    }
                }
            }else{println!("error: transition {} not found in OCPN", transition_id);}

        }

        //if breakting condition is provided (binding_sequence is Some), and it has not returned a transition yet, return empty vector
        if let Some(_) = breaking_transitions {
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

    ///removes first binding that is enabled in the marking
    pub fn remove_next_binding_(&self, binding_sequence: &mut Vec<(String, HashMap<String,HashSet<String>>)>, marking: &Marking) -> Option<(String, HashMap<String, HashSet<String>>)> { //activity, [object_types: object_ids]>

        if binding_sequence.is_empty() { return None }
        
        //TODO: test:

        //check if first binding is enabled in the marking
        let enabled_transitions = self.get_enabled_transitions_from_marking(marking);
        if enabled_transitions.is_empty() { return None }
        for transition_id in enabled_transitions {
            if self.transitions.get(&transition_id).unwrap().activity == binding_sequence.first().unwrap().0 {
                //if the first binding is enabled, return it
                return Some(binding_sequence.remove(0));
            }
        }
        return None;
        
        
        //----------------

        //Binding Sequence: <activity, [object_type: object_ids]>
        let transitions_of_binding:HashSet<String> = binding_sequence.iter().map(|(transition, _)| transition.clone()).collect();
        //get first enabled transition from the marking
        let enabled_transition = self.get_enabled_transitions_with_breakcondition(&marking, Some(&transitions_of_binding));
        //DEBUG:
        if enabled_transition.len() > 1 { println!("FAULT: too many transtions found !!!! ({})", enabled_transition.len());}
        if enabled_transition.is_empty() {
            //no enabled transition found
            return None;
        }
        //remove and return the first binding that is enabled in the marking
        if let Some(pos) = binding_sequence.iter().position(|(act, _)| act == &self.transitions[&enabled_transition.first().unwrap().0].activity) {
            let x = Some(binding_sequence.remove(pos));
            return x;
        }
        None
    }
    
    ///Retutn Vec of bindings: Transition_id, [object_type: object_ids]
    pub fn get_enabled_silent_transitions_bindings_(&self, marking: &Marking) -> Vec<(String, HashMap<String,HashSet<String>>)> {
        //return all possible states 

        let mut enabled_silent_transitions: Vec<(String, HashMap<String,HashSet<String>>)> = vec![]; //result
        let mut associated_silent_transitions: Vec<(String, HashSet<String>)> = vec![]; //transition_id, object ids that are involved TODO

        //if marking.len() <= self.silent_transtions.len() { almost always true
            //iterate over marking => is smaller
            for (place_id, objects) in marking.iter() {
                //check if this place has a silent transition
                for transition_ in self.places.get(place_id).unwrap().output_transitions.iter(){
                    
                    let transition = self.silent_transtions.get(transition_.0.as_str());
                    if transition.is_none() { continue; }  //not a silent transition

                    let mut is_enabled = true;
                    let mut temp_marking = marking.clone();
                    let mut involved_objects: HashMap<String, HashSet<String>> = HashMap::new(); //objects that are involved in this transition
                    for (input,_) in transition.unwrap().input_places.iter() {
                        

                        if !temp_marking.contains_key(input) {
                            is_enabled = false;
                            break
                        }else{
                            for object in temp_marking.clone().get(input).unwrap().iter(){
                                involved_objects.entry(self.object_to_type.get(object).unwrap().clone()).or_insert(HashSet::new()).insert(object.clone()); //add object to involved objects
                                if temp_marking.get(input).unwrap().is_empty() {
                                    temp_marking.remove(input); //if place is empty, remove it from the marking
                                }
                            }
                        }
                    }
                    //TODO: if every input is satisfied, the whole transiiton must be satisfied
                    if is_enabled {
                        //add transition to enabled silent transitions
                        enabled_silent_transitions.push((transition_.0.clone(), involved_objects.clone()));
                    }
                }
            }
        
        /*
        }else{
            for (id, transition) in self.silent_transtions.iter() {
                //check if this transition is enabled
                let mut is_enabled = true;
                let mut temp_marking = marking.clone();
                for (input,_) in transition.input_places.iter(){
                    
                    let mut involved_objects: HashMap<String, HashSet<String>> = HashMap::new(); //objects that are involved in this transition
                    
                    if !temp_marking.contains_key(input) {
                        is_enabled = false;
                        break
                    }else{
                        
                        for object in temp_marking.clone().get(input).unwrap().iter(){
                            involved_objects.entry(self.object_to_type.get(object).unwrap().clone()).or_insert(HashSet::new()).insert(object.clone()); //add object to involved objects
                            if temp_marking.get(input).unwrap().is_empty() {
                                temp_marking.remove(input); //if place is empty, remove it from the marking
                            }
                        }
                        
                        /*
                        let object = temp_marking.get_mut(input).unwrap().iter().next().cloned().map(|obj|  temp_marking
                            .get_mut(input)
                            .unwrap()
                            .take(&obj).unwrap()).unwrap(); //remove one object from the marking
                        
                        involved_objects.entry(self.object_to_type.get(&object).unwrap().clone()).or_insert(HashSet::new()).insert(object.clone()); //add object to involved objects
                        if temp_marking.get(input).unwrap().is_empty() { 
                            temp_marking.remove(input); //if place is empty, remove it from the marking
                        } 
                        
                         */
                    }
                    //TODO: if every input is satisfied, the whole transiiton must be satisfied
                    if is_enabled {
                        //add transition to enabled silent transitions
                        enabled_silent_transitions.push((id.clone(), involved_objects.clone()));
                    }
                }
            }
        }
    */

        enabled_silent_transitions
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

    pub fn execute_binding_(&self, binding: &mut (String, HashMap<String, HashSet<String>>), marking: &Marking) -> Marking{
        ///binding must be executable
        //Binding: (activity, [object_types: object_ids])
        let mut res_marking = marking.clone(); //clone marking to modify it
        let mut temp_binding = binding.clone(); //clone binding to modify it
        //TODO: what to do if there are mutliple transitions
        //TODO each transition is added doubeld to activity_to_transitions!
        for transition_id in self.activity_to_transitions.get(&binding.0).unwrap_or(&vec![]) {
            let mut silent_transition = false;
            //remove objects from input places TODO: evt remove the silent transiiton part
            for (place_id, object_type) in self.transitions.get(transition_id).unwrap_or_else(||{silent_transition = true; self.silent_transtions.get(&binding.0).unwrap()}).input_places.clone() {
                
                //if silent transition, all posssible objects have to be moved from the marking (there are no multiple arcs)
                if silent_transition {
                    for (object_type, objs) in temp_binding.1.iter_mut() {
                        for object in objs.iter(){
                            res_marking.get_mut(&place_id).expect(
                                format!("Failed to get place_id {}", place_id).as_str())
                                .remove(object); //remove from marking
                        }
                    }
                }else{
                    //continue normaly
                    if let Some(objs) = temp_binding.1.get_mut(&object_type) {
                        if let Some(object_id) = objs.iter().next().cloned() {
                            objs.remove(&object_id); // remove from temp binding
                            res_marking
                                .get_mut(&place_id)
                                .unwrap()
                                .remove(&object_id); //remove from marking
                        }
                    }
                }


                
                
                //if place is empty, remove it from the marking
                if res_marking.get(&place_id).unwrap().is_empty() {
                    res_marking.remove(&place_id);
                }
            }
            //add objects from output places
            for (place_id, object_type) in self.transitions.get(transition_id).unwrap_or_else(||self.silent_transtions.get(&binding.0).unwrap()).output_places.iter() {
                res_marking.entry(place_id.clone()).or_insert_with(HashSet::new)
                    .extend(binding.1.get(object_type).unwrap().clone());
            }
            break;
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
    pub object_type: String, //object type of the place
    pub is_initial: bool,
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

pub fn hash_marking(marking: &Marking, object_to_type: &HashMap<String, String>) -> u64 {
    //TODO: assumption: 2 objects with same id in the same place are not allowed => are only considered as one object is solved by HashSet?
    /*
    let mut hasher = DefaultHasher::new();

    let mut sorted_places: Vec<_> = marking.iter().collect();
    sorted_places.sort_by_key(|(place, _)| *place);

    for (place, objects) in sorted_places {
        
        place.hash(&mut hasher);

        let mut sorted_objs: Vec<_> = objects.iter().collect();
        sorted_objs.sort(); 

        for obj in sorted_objs {
            obj.hash(&mut hasher);
        }
    }

    hasher.finish()
    
     */


    let mut hasher = DefaultHasher::new();

    // Sort for consistent hashing
    let mut sorted_places: Vec<_> = marking.iter().collect();
    sorted_places.sort_by_key(|(place, _)| *place);

    for (place, objects) in sorted_places {
        place.hash(&mut hasher);

        // count objects per type
        let mut type_counts: BTreeMap<&String, usize> = BTreeMap::new();
        for obj in objects {
            if let Some(obj_type) = object_to_type.get(obj) {
                *type_counts.entry(obj_type).or_insert(0) += 1;
            }
        }

        
        for (obj_type, count) in type_counts {
            obj_type.hash(&mut hasher);
            count.hash(&mut hasher);
        }
    }

    hasher.finish()
}

pub fn print_marking(markings: &Vec<&Marking>) {
    //print marking
    for marking in markings {
        for (place, objects) in marking.iter() {
            println!("Place: {}, Objects: {:?}", place, objects);
        }
        println!("-------------------");
    }
}

