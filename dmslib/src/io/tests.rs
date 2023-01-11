use super::*;

#[test]
fn basic() {
    let data = r#"
        {
            "name": "John Doe",
            "branches": [
                { "nodes": [0, 1] },
                { "nodes": [1, 2] },
                { "nodes": [2, 3] }
            ],
            "externalBranches": [
                {
                    "source": 0,
                    "node": 0,
                    "status": 1
                }
            ],
            "nodes": [
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5,
                    "name": "Küçükçamlıca #3",
                    "status": 0
                },
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5
                },
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5
                }
            ],
            "resources": [
                {
                    "latlng": [
                        41.01559155019519,
                        29.092054367065433
                    ],
                    "type": null
                }
            ],
            "view": {
                "lat": 41.01303340479826,
                "lng": 29.079051017761234
            },
            "zoom": 15
        }"#;

    // Parse the string of data into serde_json::Value.
    let v: Graph = serde_json::from_str(data).unwrap();
    assert_eq!(v.name, "John Doe");

    assert_eq!(v.branches.len(), 3);

    assert_eq!(v.branches[0].nodes.0, 0);
    assert_eq!(v.branches[0].nodes.1, 1);

    assert_eq!(v.branches[1].nodes.0, 1);
    assert_eq!(v.branches[1].nodes.1, 2);

    assert_eq!(v.branches[2].nodes.0, 2);
    assert_eq!(v.branches[2].nodes.1, 3);

    assert_eq!(v.external.len(), 1);

    assert_eq!(v.nodes.len(), 3);
    assert_eq!(v.nodes[0].pf, 0.5);
    assert_eq!(v.nodes[1].pf, 0.5);
    assert_eq!(v.nodes[2].pf, 0.5);

    assert_eq!(v.nodes[0].latlng.0, v.nodes[1].latlng.0);
    assert_eq!(v.nodes[0].latlng.1, v.nodes[1].latlng.1);
}
