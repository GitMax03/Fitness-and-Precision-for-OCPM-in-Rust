extern crate process_mining as pm;


//Event Object Graph
pub struct EOG{
    //Vertex = Event ID
    pub vertices: Vec<String>,
    //edges = (Event ID, Event ID)
    pub edges: Vec<(String, String)>,
}