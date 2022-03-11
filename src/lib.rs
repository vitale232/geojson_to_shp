use std::error::Error;
use std::fs::File;

use geojson::{FeatureCollection, GeoJson, Value};
use shapefile::{
    dbase::{FieldName, TableWriter, TableWriterBuilder},
    ShapeWriter,
};

pub struct FeatureCollectionToShpWriter {
    feature_collection: FeatureCollection,
    shape_writer: ShapeWriter<File>,
    dbf_writer: TableWriter<File>,
}

impl FeatureCollectionToShpWriter {
    pub fn new(
        contents: String,
        filepath: &str,
    ) -> Result<FeatureCollectionToShpWriter, Box<dyn Error>> {
        let geojson = contents.parse::<GeoJson>()?;
        let feature_collection = match geojson {
            GeoJson::FeatureCollection(collection) => collection,
            _ => panic!("FeatureCollections only!"),
        };

        let shp = File::create(format!("{}.shp", &filepath))?;
        let shx = File::create(format!("{}.shx", &filepath))?;

        let shape_writer = ShapeWriter::with_shx(shp, shx);
        let dbf_writer = build_dbf_writer(filepath, &feature_collection)?;

        Ok(FeatureCollectionToShpWriter {
            feature_collection,
            shape_writer,
            dbf_writer,
        })
    }

    pub fn write(&mut self) -> Result<(), Box<dyn Error>> {
        for feature in self.feature_collection.features.iter() {
            let geometry = match &feature.geometry {
                Some(g) => g,
                None => panic!("No geometry for this feature!"),
            };
            match &geometry.value {
                Value::Point(p) => {
                    let geom: geo_types::Point<f64> = (p[0], p[1]).try_into()?;
                    let geom: shapefile::Point = geom.try_into()?;

                    self.shape_writer.write_shape(&geom)?;
                }
                Value::LineString(line) => {
                    let points: Vec<(f64, f64)> =
                        line.iter().map(|point| (point[0], point[1])).collect();
                    let geom = geo_types::LineString::from(points);
                    let geom: shapefile::Polyline = geom.try_into()?;

                    self.shape_writer.write_shape(&geom)?;
                }
                _ => panic!("Unimplemented Geometry Type!"),
            };

            let properties = match &feature.properties {
                Some(props) => props,
                None => panic!("No properties!"),
            };
            let mut record = shapefile::dbase::Record::default();
            for (prop_name, value) in properties.into_iter() {
                match value {
                    serde_json::Value::Number(val) => {
                        record.insert(
                            prop_name.to_string(),
                            shapefile::dbase::FieldValue::Numeric(val.as_f64()),
                        );
                    }
                    serde_json::Value::String(val) => {
                        record.insert(
                            prop_name.to_string(),
                            shapefile::dbase::FieldValue::Character(Some(val.to_string())),
                        );
                    }
                    _ => panic!("lazy"),
                }
            }
            self.dbf_writer
                .write_record(&record)
                .expect("Could not write record!");
        }
        Ok(())
    }
}

fn build_dbf_writer(
    filepath: &str,
    feature_collection: &FeatureCollection,
) -> Result<TableWriter<File>, Box<dyn Error>> {
    let mut writer = TableWriterBuilder::new();
    let feature = feature_collection.features[0].clone();
    let properties = feature
        .properties
        .expect("No properties exist on the feature!");

    for (prop_name, value) in properties.iter() {
        match value {
            serde_json::Value::Number(_) => {
                writer = writer.add_numeric_field(FieldName::try_from(&prop_name[..])?, 22, 20)
            }
            serde_json::Value::String(_) => {
                writer = writer.add_character_field(FieldName::try_from(&prop_name[..])?, 255);
            },
            _ => panic!("Property type not supported! Only Number and String values are currently supported.")
        }
    }
    let dest = File::create(format!("{}.dbf", filepath))?;
    Ok(writer.build_with_dest(dest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_converter_and_converts_without_error() {
        let contents = std::fs::read_to_string("./fixtures/points.geojson").unwrap();
        let mut writer = FeatureCollectionToShpWriter::new(contents, "./fixtures/test").unwrap();
        writer.write().expect("Shapes")
    }
}
