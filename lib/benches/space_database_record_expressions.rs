
#![feature(test)]

extern crate test;

use test::{Bencher, black_box};

use std::time::{Instant};

use std::fs::File;
use std::io::BufReader;

use serde::*;
use csv::ReaderBuilder;

use hyperon::*;
use hyperon::space::grounding::*;

//Specify the test file path to run this benchmark
#[ignore]
#[bench]
fn database_record_expressions(bencher: &mut Bencher) -> std::io::Result<()> {

    let mut space = GroundingSpace::new();

    // A geonames file may be downloaded from: [http://download.geonames.org/export/dump/cities500.zip]
    // for a large file, or "cities15000.zip" for a smaller file, depending on the characteristics of the
    // benchmark you want
    let file = File::open("/Users/admin/Desktop/cities500.txt")?;

    //Data structure to parse the GeoNames TSV file into
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct GeoName {
        geonameid         : i32, //integer id of record in geonames database
        name              : String, //name of geographical point (utf8) varchar(200)
        asciiname         : String, //name of geographical point in plain ascii characters, varchar(200)
        alternatenames    : String, //alternatenames, comma separated, ascii names automatically transliterated, convenience attribute from alternatename table, varchar(10000)
        latitude          : f32, //latitude in decimal degrees (wgs84)
        longitude         : f32, //longitude in decimal degrees (wgs84)
        feature_class     : char, //see http://www.geonames.org/export/codes.html, char(1)
        feature_code      : String,//[char; 10], //see http://www.geonames.org/export/codes.html, varchar(10)
        country_code      : String,//[char; 2], //ISO-3166 2-letter country code, 2 characters
        cc2               : String, //alternate country codes, comma separated, ISO-3166 2-letter country code, 200 characters
        admin1_code       : String,//[char; 20], //fipscode (subject to change to iso code), see exceptions below, see file admin1Codes.txt for display names of this code; varchar(20)
        admin2_code       : String, //code for the second administrative division, a county in the US, see file admin2Codes.txt; varchar(80) 
        admin3_code       : String,//[char; 20], //code for third level administrative division, varchar(20)
        admin4_code       : String,//[char; 20], //code for fourth level administrative division, varchar(20)
        population        : i64, //bigint (8 byte int)
        #[serde(deserialize_with = "default_if_empty")]
        elevation         : i32, //in meters, integer
        #[serde(deserialize_with = "default_if_empty")]
        dem               : i32, //digital elevation model, srtm3 or gtopo30, average elevation of 3''x3'' (ca 90mx90m) or 30''x30'' (ca 900mx900m) area in meters, integer. srtm processed by cgiar/ciat.
        timezone          : String, //the iana timezone id (see file timeZone.txt) varchar(40)
        modification_date : String, //date of last modification in yyyy-MM-dd format
    }
    fn default_if_empty<'de, D, T>(de: D) -> Result<T, D::Error>
        where D: serde::Deserializer<'de>, T: serde::Deserialize<'de> + Default,
    {
        Option::<T>::deserialize(de).map(|x| x.unwrap_or_else(|| T::default()))
    }

    //TODO: LP: This might be a Yak-shave, but I'd like to make a serde format for atoms,
    // so I could just serialize this structure into an atom.  Of course then I'd want to define attributes
    // that allow me to configure stuff like whether to include field names (and rename fields if so),
    // and it would end up being 1000 lines of code and take a week before I was happy with it. :-/
    fn expr_from_geoname(geoname: GeoName) -> Atom {
        Atom::expr([
            sym!("geoname"),
            Atom::sym(geoname.name),
            Atom::sym(geoname.asciiname),
            Atom::sym(geoname.alternatenames),
            Atom::sym(geoname.latitude.to_string()),
            Atom::sym(geoname.longitude.to_string()),
            Atom::sym(geoname.feature_class),
            Atom::sym(geoname.feature_code),
            Atom::sym(geoname.country_code),
            Atom::sym(geoname.cc2),
            Atom::sym(geoname.admin1_code),
            Atom::sym(geoname.admin2_code),
            Atom::sym(geoname.admin3_code),
            Atom::sym(geoname.admin4_code),
            Atom::sym(geoname.population.to_string()),
            Atom::sym(geoname.elevation.to_string()),
            Atom::sym(geoname.dem.to_string()),
            Atom::sym(geoname.timezone),
            Atom::sym(geoname.modification_date),
        ])
    }

    //Parser for the tab-saparated value file
    let reader = BufReader::new(file);
    let mut tsv_parser = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true) //We want to permit situations where some rows have fewer columns for now
        .quote(0)
        .double_quote(false)
        .from_reader(reader);

    let mut expr_count = 0;
    let mut tsv_record_count = 0;
    let start = Instant::now();
    for geoname in tsv_parser.deserialize::<GeoName>().map(|result| result.unwrap()) {
        tsv_record_count += 1;

        if geoname.alternatenames.len() > 0 {
            //Separate the comma-separated alternatenames field
            for alt_name in geoname.alternatenames.split(',') {
                let mut geoname = geoname.clone();
                geoname.alternatenames = alt_name.to_owned();
                let expr = expr_from_geoname(geoname);
                space.add(expr);
                expr_count += 1;
            }
        } else {
            let expr = expr_from_geoname(geoname);
            space.add(expr);
            expr_count += 1;
        }
    }

    //Space-building Stats.  Run with `cargo bench -- --nocapture` to see the results
    //NOTE: The time taken to parse the file is currently a tiny fraction compared with adding the expression
    // to the space.  But if this changes in the future this part of the benchmark will become unreliable.
    println!("tsv_record_count = {tsv_record_count}");
    println!("expr_count = {expr_count}");
    let end = Instant::now();
    let elapsed = end - start;
    println!("time elapsed building space: {:.3} seconds, {:.3} µs/expr", (elapsed.as_millis() as f64 / 1000.00), (elapsed.as_nanos() as f64/expr_count as f64/1000.00));

    let query_expr_1 = &expr!("geoname" Name AsciiName "Tokyo" Lat Lon FeatureClass FetureCode CountryCode CC2 Admin1 Admin2 Admin3 Admin4 Pop Elev Dem TZ ModDate);
    let reference_binding_1 = bind_set![{ Name: sym!("Tokyo"), AsciiName: sym!("Tokyo"),
        Lat: sym!("35.6895"), Lon: sym!("139.69171"), FeatureClass: sym!("P"), FetureCode: sym!("PPLC"),
        CountryCode: sym!("JP"), CC2: sym!(""), Admin1: sym!("40"), Admin2: sym!(""), Admin3: sym!(""), Admin4: sym!(""),
        Pop: sym!("8336599"), Elev: sym!("0"), Dem: sym!("44"), TZ: sym!("Asia/Tokyo"), ModDate: sym!("2022-11-25") }];

    bencher.iter(|| {
        let result_binding = black_box(space.query(query_expr_1));
        assert_eq!(result_binding, reference_binding_1);
    });

    Ok(())
}
