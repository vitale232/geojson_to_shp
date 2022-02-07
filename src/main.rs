use core::panic;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::{env::current_dir, error::Error};

use geojson::{GeoJson, Value};
use shapefile::dbase::{
    FieldIOError, FieldName, FieldWriter, TableWriter, TableWriterBuilder, WritableRecord,
};
use shapefile::{Point, ShapeWriter};

struct MyFeature {
    category: String,
}

impl WritableRecord for MyFeature {
    fn write_using<'a, W: Write>(
        &self,
        field_writer: &mut FieldWriter<'a, W>,
    ) -> Result<(), FieldIOError> {
        field_writer.write_next_field_value(&self.category)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct JsonFieldList {
    values: Vec<serde_json::Value>,
    field_name: String,
}

#[derive(Clone, Debug)]
struct Converter {
    fields: Vec<JsonFieldList>,
}

impl Converter {
    pub fn new() -> Converter {
        Converter { fields: Vec::new() }
    }

    fn get_index(&mut self, field_name: String) -> Option<usize> {
        self.fields.iter().position(|f| f.field_name == field_name)
    }

    fn get_field_list(&mut self, field_name: String) -> Option<&JsonFieldList> {
        let index = self.get_index(field_name)?;
        self.fields.get(index)
    }

    fn get_field_name(&mut self, index: usize) -> Option<String> {
        let field = self.fields.get(index)?;
        Some(field.field_name.clone())
    }

    pub fn insert_field(&mut self, field_name: String) -> &Self {
        let index = self.get_index(field_name.clone());
        match index {
            Some(_) => self,
            None => {
                let _ = &self.fields.push(JsonFieldList {
                    field_name,
                    values: Vec::new(),
                });
                self
            }
        }
    }

    pub fn insert_val(
        &mut self,
        field_name: String,
        value: serde_json::Value,
    ) -> Result<&Self, Box<dyn Error>> {
        let index = self.get_index(field_name.clone());
        let is_existing_field = match index {
            Some(_) => true,
            None => false,
        };
        if is_existing_field == false {
            self.insert_field(field_name.clone());
        }
        let field_index = self.get_index(field_name);
        match field_index {
            Some(idx) => {
                self.fields[idx].values.push(value);
                Ok(self)
            }
            None => panic!("Could not insert the value!"),
        }
    }
}

fn read_geojson(filepath: &str) -> Result<GeoJson, Box<dyn Error>> {
    let contents = std::fs::read_to_string(filepath)?;
    let geojson = contents.parse::<GeoJson>()?;
    Ok(geojson)
}

fn write_shapes(filepath: &str, shapes: &Vec<Point>) -> Result<(), shapefile::Error> {
    let shp = File::create(format!("{}.shp", &filepath))?;
    let shx = File::create(format!("{}.shx", &filepath))?;

    let writer = ShapeWriter::with_shx(shp, shx);
    writer.write_shapes(&*shapes)?;
    Ok(())
}

fn build_dbf_writer(
    filepath: &str,
    fields: Vec<JsonFieldList>,
) -> Result<TableWriter<File>, Box<dyn Error>> {
    {
        let mut writer = TableWriterBuilder::new();
        for field in fields {
            match field.values[0] {
                serde_json::Value::Array(_) => panic!("Arrays are not supported!"),
                serde_json::Value::Object(_) => panic!("ob is not supported"),
                serde_json::Value::Null => panic!("null is not supported"),
                serde_json::Value::Bool(_) => panic!("bool is not supported"),
                serde_json::Value::Number(_) => {
                    writer = writer.add_numeric_field(
                        FieldName::try_from(&field.field_name[..])?,
                        22,
                        20,
                    )
                }
                serde_json::Value::String(_) => {
                    writer = writer
                        .add_character_field(FieldName::try_from(&field.field_name[..])?, 255);
                }
            }
        }
        let dest = File::create(format!("{}.dbf", filepath))?;
        Ok(writer.build_with_dest(dest))
    }
}

fn main() {
    println!(
        "Current directory: {:?}",
        current_dir().expect("OS cannot access current directory?!")
    );
    let geojson = read_geojson("./fixtures/points.geojson").expect("`points.geojson` read failed.");

    let mut converter = Converter::new();
    let mut geom_vec = Vec::new();

    match &geojson {
        GeoJson::Feature(_) => panic!("Type `GeoJson::Feature` is not supported!"),
        GeoJson::Geometry(_) => panic!("Type `GeoJson::Geometry` is not supported!"),
        GeoJson::FeatureCollection(collection) => {
            for feature in collection.features.iter() {
                match &feature.geometry {
                    Some(g) => match &g.value {
                        Value::Point(p) => {
                            println!("\n  geojson point: {:?}", feature.geometry);
                            let geom: geo_types::Point<f64> = (p[0], p[1]).try_into().unwrap();
                            println!("  geo-types point: {:?}", geom);
                            let geom: shapefile::Point = geom.try_into().unwrap();
                            println!("  shapefile point: {:?}", geom);

                            geom_vec.push(geom);
                        }
                        Value::MultiPoint(mp) => println!("multi-point: {:?}", mp),
                        Value::LineString(ls) => println!("ls: {:?}", ls),
                        Value::MultiLineString(mls) => println!("mls: {:?}", mls),
                        Value::Polygon(poly) => println!("poly: {:?}", poly),
                        Value::MultiPolygon(multi_poly) => {
                            println!("multi_poly: {:?}", multi_poly)
                        }
                        Value::GeometryCollection(gc) => println!("gc: {:?}", gc),
                    },
                    None => panic!("The Geometry is empty!"),
                }
                match &feature.properties {
                    Some(props) => {
                        println!("  props:");
                        for (prop_name, value) in props.into_iter() {
                            println!("    {}: {}", prop_name, value);
                            match value {
                                serde_json::Value::Array(_) => {
                                    panic!("Arrays are not supported!")
                                }
                                serde_json::Value::Bool(_) => {
                                    panic!("bool is not supported")
                                }
                                serde_json::Value::Object(_) => panic!("ob is not supported"),
                                serde_json::Value::Null => panic!("null is not supported"),
                                serde_json::Value::Number(_) => {
                                    converter.insert_field(prop_name[..].to_string());
                                    converter
                                        .insert_val(prop_name[..].to_string(), value.to_owned())
                                        .unwrap();
                                }
                                serde_json::Value::String(_) => {
                                    converter.insert_field(prop_name[..].to_string());
                                    converter
                                        .insert_val(prop_name[..].to_string(), value.to_owned())
                                        .unwrap();
                                }
                            }
                        }
                    }
                    None => println!("  No Props!"),
                }
            }
        }
    };
    println!(
        "\ngeom_vec: {:?}\ngeom_vec len: {}",
        geom_vec,
        geom_vec.len()
    );
    write_shapes("./fixtures/garbage_out", &geom_vec).expect("Write failed!");
    let mut writer = build_dbf_writer("fixtures/garbage_out", converter.clone().fields)
        .expect("Couldn't not make DBF Writer 2");

    println!("Converter: {:?}", converter);

    for val_index in 0..converter.fields[0].values.len() {
        let mut record = shapefile::dbase::Record::default();
        for field_index in 0..converter.fields.len() {
            let field_name = converter.get_field_name(field_index).unwrap();
            let field_list = converter.get_field_list(field_name).unwrap();
            let val = &field_list.values[val_index];
            match val {
                serde_json::Value::Array(_) => panic!("Arrays are not supported!"),
                serde_json::Value::Bool(_) => panic!("bool is not supported"),
                serde_json::Value::Object(_) => panic!("ob is not supported"),
                serde_json::Value::Null => panic!("null is not supported"),
                serde_json::Value::Number(_) => {
                    record.insert(
                        field_list.field_name.to_string(),
                        shapefile::dbase::FieldValue::Numeric(val.as_f64()),
                    );
                }
                serde_json::Value::String(_) => {
                    record.insert(
                        field_list.field_name.to_string(),
                        shapefile::dbase::FieldValue::Character(Some(val.to_string())),
                    );
                }
            }
        }
        writer
            .write_record(&record)
            .expect("Couldn't write `record`!");
    }
}
