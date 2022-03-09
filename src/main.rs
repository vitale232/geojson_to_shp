mod converter;

fn main() {
    let contents = std::fs::read_to_string("./fixtures/lines.geojson").unwrap();
    let mut converter =
        converter::FeatureCollectionToShpConverter::new(contents, "./fixtures/here").unwrap();

    converter
        .write_shapefile()
        .expect("Converter conversion collapsed!")
}
