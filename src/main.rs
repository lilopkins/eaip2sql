use std::sync::Arc;

use airac::AIRAC;
use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use eaip::{eaip::ais, prelude::*};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sqlx::AnyPool;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// The connection URL for the database. MySQL and SQLite supported.
    #[clap(short, long, default_value = "sqlite:navdata.db")]
    database_uri: String,

    /// Build data for the next AIRAC cycle, instead of the current one.
    #[clap(short, long)]
    next_cycle: bool,

    /// AIS sources to exclude by two letter country code.
    #[clap(short = 'x', long)]
    exclude_ais: Vec<String>,

    /// List all available AIS sources.
    #[clap(short, long)]
    list_ais: bool,
}

async fn get_eaip_data(
    m: &MultiProgress,
    pb_eaip: &ProgressBar,
    sty: ProgressStyle,
    eaip: &EAIP,
    airac: AIRAC,
    pool: &AnyPool,
) -> Result<()> {
    pb_eaip.set_position(0);
    pb_eaip.set_message("Fetching navaids... ");
    let navaids = Navaids::from_eaip(eaip, airac.clone()).await.unwrap();
    pb_eaip.inc(1);

    pb_eaip.set_message("Storing navaids... ");
    let mut already_inserted = Vec::new();
    for navaid in navaids {
        if already_inserted.contains(navaid.id()) {
            continue;
        }
        already_inserted.push(navaid.id().clone());

        pb_eaip.set_message(format!("Storing navaid {}... ", navaid.id()));
        sqlx::query("INSERT INTO `navaid` (`id`, `name`, `frequency`, `latitude`, `longitude`, `elevation`, `type`) VALUES (?, ?, ?, ?, ?, ?, ?);")
            .bind(navaid.id())
            .bind(navaid.name())
            .bind(if navaid.kind() == NavAidKind::NDB { navaid.frequency_khz() as f32 } else { navaid.frequency() })
            .bind(navaid.latitude())
            .bind(navaid.longitude())
            .bind(navaid.elevation() as i32)
            .bind(match navaid.kind() {
                NavAidKind::VOR => "VOR",
                NavAidKind::DME => "DME",
                NavAidKind::NDB => "NDB",
                NavAidKind::VORDME => "VOR,DME",
            })
            .execute(pool)
            .await
            .with_context(|| format!("Inserting navaid {} to the database.", navaid.id()))?;
    }
    pb_eaip.inc(1);

    pb_eaip.set_message("Fetching intersections... ");
    let intersections = Intersections::from_eaip(eaip, airac.clone()).await.unwrap();
    pb_eaip.inc(1);
    pb_eaip.set_message("Storing intersections... ");
    for intersection in intersections {
        pb_eaip.set_message(format!(
            "Storing intersection {}... ",
            intersection.designator()
        ));
        sqlx::query(
            "INSERT INTO `intersection` (`designator`, `latitude`, `longitude`) VALUES (?, ?, ?);",
        )
        .bind(intersection.designator())
        .bind(intersection.latitude())
        .bind(intersection.longitude())
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "Inserting intersection {} to the database.",
                intersection.designator()
            )
        })?;
    }
    pb_eaip.inc(1);

    pb_eaip.set_message("Fetching airways... ");
    let airways = Airways::from_eaip(eaip, airac.clone()).await.unwrap();
    pb_eaip.inc(1);
    pb_eaip.set_message("Storing airways... ");
    for airway in airways {
        let mut i = 1;
        pb_eaip.set_message(format!("Storing {} airway... ", airway.designator()));
        for waypoint in airway.waypoints() {
            sqlx::query(
                "INSERT INTO `airway_waypoint` (`airway_designator`, `waypoint_id`, `upper_limit`, `lower_limit`, `navaid_id`, `intersection_designator`) VALUES (?, ?, ?, ?, ?, ?);",
            )
            .bind(airway.designator())
            .bind(i)
            .bind(waypoint.upper_limit())
            .bind(waypoint.lower_limit())
            .bind(if waypoint.is_navaid() { Some(waypoint.designator()) } else { None })
            .bind(if waypoint.is_intersection() { Some(waypoint.designator()) } else { None })
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "Inserting airway {} waypoint {} to the database.",
                    airway.designator(),
                    waypoint.designator(),
                )
            })?;
            i += 1;
        }
    }
    pb_eaip.inc(1);

    pb_eaip.set_message("Fetching airport list... ");
    let mut airports = Airports::from_eaip(eaip, airac.clone()).await.unwrap();
    pb_eaip.inc(1);

    pb_eaip.set_message("Fetching airport details... ");
    let pb_airports = m.insert_after(&pb_eaip, ProgressBar::new(airports.len() as u64));
    pb_airports.set_style(sty.clone());
    for airport in &mut airports {
        let icao = airport.icao().clone();
        pb_airports.set_message(format!("Fetching airport {}... ", icao));
        *airport = Airport::from_eaip(eaip, airac.clone(), icao).await.unwrap();
        pb_airports.inc(1);
    }
    pb_eaip.inc(1);
    pb_eaip.set_message("Storing airports... ");
    for airport in airports {
        pb_eaip.set_message(format!("Storing airport {}... ", airport.icao()));
        sqlx::query(
            "INSERT INTO `airport` (`icao`, `name`, `latitude`, `longitude`, `elevation`) VALUES (?, ?, ?, ?, ?);",
        )
        .bind(airport.icao())
        .bind(airport.name())
        .bind(airport.latitude())
        .bind(airport.longitude())
        .bind(airport.elevation() as i32)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "Inserting airport {} to the database.",
                airport.icao()
            )
        })?;

        let mut i = 1i32;
        for chart in airport.charts() {
            sqlx::query(
                "INSERT INTO `chart` (`airport_icao`, `chart_id`, `title`, `url`) VALUES (?, ?, ?, ?);",
            )
            .bind(airport.icao())
            .bind(i)
            .bind(chart.title())
            .bind(chart.url())
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "Inserting airport {} chart {} to the database.",
                    airport.icao(),
                    chart.title(),
                )
            })?;
            i += 1;
        }
    }
    pb_eaip.inc(1);

    m.remove(&pb_airports);
    Ok(())
}

async fn add_metadata_property<S: Into<String>>(pool: &AnyPool, key: S, value: S) -> Result<()> {
    let key = key.into();

    sqlx::query("INSERT INTO `properties` (`id`, `value`) VALUES (?, ?);")
        .bind(key.clone())
        .bind(value.into())
        .execute(pool)
        .await
        .with_context(|| format!("Failed to insert property {}. Maybe this database has already had navdata generated?", key))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.list_ais {
        println!("Available AISs:");
        for s in &*ais::ALL {
            println!("  {}: {}", s.country(), s.name());
        }
        std::process::exit(0);
    }

    let pool = AnyPool::connect(&args.database_uri)
        .await
        .with_context(|| "Failed to connect to database!")?;

    // Preparing database
    sqlx::query(include_str!("schema.sql"))
        .execute(&pool)
        .await
        .with_context(|| "Failed to prepare database schema.")?;

    let mut airac = AIRAC::current();
    if args.next_cycle {
        airac = airac.next();
    }

    // Add metadata to properties table
    add_metadata_property(&pool, "generator", "eaip2sql").await?;
    add_metadata_property(&pool, "valid_from", &airac.starts().to_string()).await?;
    add_metadata_property(&pool, "valid_until", &airac.ends().to_string()).await?;
    add_metadata_property(&pool, "generated_at", &Utc::now().to_string()).await?;

    let mut eaips = Vec::new();
    for eaip in &*ais::ALL {
        if !args.exclude_ais.contains(&eaip.country().to_string()) {
            eaips.push(eaip);
        }
    }

    let m = Arc::new(MultiProgress::new());
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("=> ");

    let pb = m.add(ProgressBar::new(eaips.len() as u64));
    pb.set_style(sty.clone());

    let pb_eaip = m.insert_after(&pb, ProgressBar::new(9));
    pb_eaip.set_style(sty.clone());

    let m_c = m.clone();
    let worker = tokio::spawn(async move {
        for eaip in eaips {
            pb.set_message(format!("{} ({})", eaip.name(), eaip.country()));
            let eaip = eaip.eaip();
            get_eaip_data(&m_c, &pb_eaip, sty.clone(), eaip, airac.clone(), &pool)
                .await
                .unwrap();
        }
    });

    let _ = worker
        .await
        .with_context(|| "Failed to process eAIP data.")?;
    m.clear().unwrap();

    Ok(())
}
