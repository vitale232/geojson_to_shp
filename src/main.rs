use geojson_to_shp::FeatureCollectionToShpWriter;

fn main() {
    let contents = std::fs::read_to_string("./fixtures/lines.geojson").unwrap();

    let mut writer = FeatureCollectionToShpWriter::new(contents, "./fixtures/here").unwrap();
    writer.write().expect("Converter conversion collapsed!")
}
