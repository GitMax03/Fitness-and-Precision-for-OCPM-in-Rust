use std::collections::HashSet;

pub fn has_intersection(a: &[String], b: &[String]) -> bool {
    // no _is_disjoint func for vec?
    let set_a: HashSet<_> = a.iter().collect();  
    b.iter().any(|s| set_a.contains(s))         
}