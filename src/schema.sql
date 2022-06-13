CREATE TABLE IF NOT EXISTS `properties` (
    `id` VARCHAR(128) NOT NULL,
    `value` TEXT,
    PRIMARY KEY (`id`));

CREATE TABLE IF NOT EXISTS `navaid` (
    `designator` CHAR(3) NOT NULL,
    `name` VARCHAR(45) NOT NULL,
    `frequency` DECIMAL(6,3) NOT NULL,
    `latitude` FLOAT NOT NULL,
    `longitude` FLOAT NOT NULL,
    `elevation` INT NOT NULL,
    `type` VARCHAR(32) NOT NULL DEFAULT 'VOR,DME');

CREATE TABLE IF NOT EXISTS `intersection` (
    `designator` CHAR(5) NOT NULL,
    `latitude` FLOAT NOT NULL,
    `longitude` FLOAT NOT NULL);

-- CREATE TABLE IF NOT EXISTS `airway_waypoint` (
--     `airway_designator` VARCHAR(5) NOT NULL,
--     `waypoint_id` INT NOT NULL,
--     `upper_limit` VARCHAR(16) NOT NULL,
--     `lower_limit` VARCHAR(16) NOT NULL,
--     `navaid_rowid` CHAR(3) NULL,
--     `intersection_rowid` CHAR(5) NULL,
--     PRIMARY KEY (`airway_designator`, `waypoint_id`),
--     CONSTRAINT `fk_airway_navaid`
--         FOREIGN KEY (`navaid_rowid`)
--         REFERENCES `navaid` (`rowid`)
--         ON DELETE RESTRICT
--         ON UPDATE RESTRICT,
--     CONSTRAINT `fk_airway_intersection`
--         FOREIGN KEY (`intersection_rowid`)
--         REFERENCES `intersection` (`rowid`)
--         ON DELETE RESTRICT
--         ON UPDATE RESTRICT);

CREATE TABLE IF NOT EXISTS `airport` (
    `icao` CHAR(4) NOT NULL,
    `name` VARCHAR(255) NOT NULL,
    `latitude` DOUBLE NOT NULL,
    `longitude` DOUBLE NOT NULL,
    `elevation` INT NOT NULL,
    PRIMARY KEY (`icao`));

CREATE TABLE IF NOT EXISTS `chart` (
    `airport_icao` CHAR(4) NOT NULL,
    `chart_id` INT NOT NULL,
    `title` VARCHAR(128) NOT NULL,
    `url` TEXT NOT NULL,
    PRIMARY KEY (`airport_icao`, `chart_id`),
    CONSTRAINT `fk_chart_airport`
        FOREIGN KEY (`airport_icao`)
        REFERENCES `airport` (`icao`)
        ON DELETE CASCADE
        ON UPDATE CASCADE);
        