use stembed::{
    core::{Stroke, StrokeContext, StrokeContextError},
    io::{HeapFile, Seek, SeekFrom},
    serialize::{Deserialize, Serialize},
};

#[test]
fn blub() {
    let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &["FN1", "FN2"]).unwrap();
    let stroke = Stroke::from_str("KH-PD|FN1,FN2", &context).unwrap();
    let output = stroke.to_string();
    assert_eq!(output, "KH-PD|FN1,FN2");
}

#[test]
fn survives_serialization_roundtrip() {
    let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &["FN1", "FN2"]).unwrap();
    let stroke = Stroke::from_str("KH-PD|FN1,FN2", &context).unwrap();

    let mut output = HeapFile::new();
    stroke.serialize(&mut output).unwrap();
    output.seek(SeekFrom::Start(0)).unwrap();
    let deserialized = Stroke::deserialize(&mut output, &context).unwrap();
    assert_eq!(stroke, deserialized);
}

#[test]
fn fails_on_duplicate_keys() {
    assert_eq!(StrokeContext::new("S", "A", "D", &["FN1"]).err(), None);
    assert_eq!(
        StrokeContext::new("STS", "A", "D", &["FN1"]).err(),
        Some(StrokeContextError::DuplicateKey)
    );
    assert_eq!(
        StrokeContext::new("S", "ABA", "D", &["FN1"]).err(),
        Some(StrokeContextError::DuplicateKey)
    );
    assert_eq!(
        StrokeContext::new("S", "A", "DZD", &["FN1"]).err(),
        Some(StrokeContextError::DuplicateKey)
    );
    assert_eq!(
        StrokeContext::new("S", "A", "D", &["FN1", "FN2", "FN1"]).err(),
        Some(StrokeContextError::DuplicateKey)
    );
}
