use std::{fs::File, io::prelude::*, io::BufWriter};

use eaip::{eaip::ais::GB, prelude::*};

#[tokio::main]
async fn main() {
    let eaip = &*GB;
    eprint!("Fetching navaids... ");
    let navaids = Navaids::from_current_eaip(eaip).await.unwrap();
    eprint!("done!\nFetching intersections... ");
    let intersections = Intersections::from_current_eaip(eaip).await.unwrap();
    eprint!("done!\nFetching airways... ");
    let airways = Airways::from_current_eaip(eaip).await.unwrap();
    eprint!("done!\nFetching airport list... ");
    let mut airports = Airports::from_current_eaip(eaip).await.unwrap();
    for airport in &mut airports {
        let icao = airport.icao().clone();
        eprint!("done!\nFetching airport {}... ", icao);
        *airport = Airport::from_current_eaip(eaip, icao).await.unwrap();
    }

    eprint!("done!\nBuilding SQL... ");

    let mut sql_out = BufWriter::new(File::create("navdata.sql").unwrap());

    writeln!(sql_out, "-- Navaids --").unwrap();
    writeln!(
        sql_out,
        "CREATE TABLE IF NOT EXISTS `navaid` (
        `id` CHAR(3) NOT NULL,
        `name` VARCHAR(45) NOT NULL,
        `frequency` DECIMAL(6,3) NOT NULL,
        `latitude` DOUBLE NOT NULL,
        `longitude` DOUBLE NOT NULL,
        `elevation` INT NOT NULL,
        `type` SET('VOR', 'DME', 'NDB', 'TACAN') NOT NULL DEFAULT 'VOR,DME',
        PRIMARY KEY (`id`));"
    )
    .unwrap();

    // Some duplication occurs when two navaids use the same designator, we only really care about one.
    let mut designators = Vec::new();
    for navaid in navaids {
        if designators.contains(navaid.id()) {
            continue;
        }
        designators.push(navaid.id().clone());
        writeln!(
            sql_out,
            r#"INSERT INTO `navaid` VALUES ("{}", "{}", {}, {}, {}, {}, "{}");"#,
            navaid.id(),
            navaid.name(),
            if navaid.kind() == NavAidKind::NDB {
                navaid.frequency_khz() as f32
            } else {
                navaid.frequency()
            },
            navaid.latitude(),
            navaid.longitude(),
            navaid.elevation(),
            match navaid.kind() {
                NavAidKind::DME => "DME",
                NavAidKind::NDB => "NDB",
                NavAidKind::VOR => "VOR",
                NavAidKind::VORDME => "VOR,DME",
            }
        )
        .unwrap();
    }

    writeln!(sql_out, "\n\n-- Intersections --").unwrap();
    writeln!(
        sql_out,
        "CREATE TABLE IF NOT EXISTS `intersection` (
        `designator` CHAR(5) NOT NULL,
        `latitude` DOUBLE NOT NULL,
        `longitude` DOUBLE NOT NULL,
        PRIMARY KEY (`designator`));"
    )
    .unwrap();

    for intersection in intersections {
        writeln!(
            sql_out,
            r#"INSERT INTO `intersection` VALUES ("{}", {}, {});"#,
            intersection.designator(),
            intersection.latitude(),
            intersection.longitude(),
        )
        .unwrap();
    }

    writeln!(sql_out, "\n\n-- Airways --").unwrap();
    writeln!(
        sql_out,
        "CREATE TABLE IF NOT EXISTS `airway_waypoint` (
        `airway_designator` VARCHAR(5) NOT NULL,
        `waypoint_id` INT NOT NULL,
        `upper_limit` VARCHAR(16) NOT NULL,
        `lower_limit` VARCHAR(16) NOT NULL,
        `navaid_id` CHAR(3) NULL,
        `intersection_designator` CHAR(5) NULL,
        PRIMARY KEY (`airway_designator`, `waypoint_id`),
        INDEX `fk_airway_navaid_idx` (`navaid_id` ASC) VISIBLE,
        INDEX `fk_airway_intersection_idx` (`intersection_designator` ASC) VISIBLE,
        CONSTRAINT `fk_airway_navaid`
          FOREIGN KEY (`navaid_id`)
          REFERENCES `navaid` (`ID`)
          ON DELETE RESTRICT
          ON UPDATE RESTRICT,
        CONSTRAINT `fk_airway_intersection`
          FOREIGN KEY (`intersection_designator`)
          REFERENCES `intersection` (`designator`)
          ON DELETE RESTRICT
          ON UPDATE RESTRICT);"
    )
    .unwrap();

    for airway in airways {
        let mut i = 1;
        for waypoint in airway.waypoints() {
            writeln!(
                sql_out,
                r#"INSERT INTO `airway_waypoint` VALUES ("{}", {}, "{}", "{}", {}, {});"#,
                airway.designator(),
                i,
                waypoint.upper_limit().replace("\n", ""),
                waypoint.lower_limit().replace("\n", ""),
                if waypoint.is_navaid() { format!(r#""{}""#, waypoint.designator()) } else { "NULL".into() },
                if waypoint.is_intersection() { format!(r#""{}""#, waypoint.designator()) } else { "NULL".into() }
            )
            .unwrap();
            i += 1;
        }
    }

    writeln!(sql_out, "\n\n-- Airports --").unwrap();
    writeln!(
        sql_out,
        "CREATE TABLE IF NOT EXISTS `airport` (
            `icao` CHAR(4) NOT NULL,
            `name` VARCHAR(255) NOT NULL,
            `latitude` DOUBLE NOT NULL,
            `longitude` DOUBLE NOT NULL,
            `elevation` INT NOT NULL,
            PRIMARY KEY (`icao`));"
    )
    .unwrap();
    writeln!(
        sql_out,
        "CREATE TABLE IF NOT EXISTS `chart` (
            `id` INT NOT NULL AUTO_INCREMENT,
            `airport_icao` CHAR(4) NOT NULL,
            `title` VARCHAR(128) NOT NULL,
            `url` TEXT NOT NULL,
            PRIMARY KEY (`id`),
            INDEX `fk_chart_airport_idx` (`airport_icao` ASC) VISIBLE,
            CONSTRAINT `fk_chart_airport`
              FOREIGN KEY (`airport_icao`)
              REFERENCES `airport` (`icao`)
              ON DELETE CASCADE
              ON UPDATE CASCADE);"
    )
    .unwrap();

    for airport in airports {
        writeln!(
            sql_out,
            r#"INSERT INTO `airport` VALUES ("{}", "{}", {}, {}, {});"#,
            airport.icao(),
            airport.name(),
            airport.longitude(),
            airport.latitude(),
            airport.elevation(),
        )
        .unwrap();
        for chart in airport.charts() {
            writeln!(
                sql_out,
                r#"INSERT INTO `chart` (`airport_icao`, `title`, `url`) VALUES ("{}", "{}", "{}");"#,
                airport.icao(),
                chart.title(),
                chart.url(),
            )
            .unwrap();
        }
    }

    eprintln!("done!");
}
