-- Create Properties Table
CREATE TABLE Properties(
  id serial PRIMARY KEY,
  apn VARCHAR(50) UNIQUE NOT NULL,
  street_number VARCHAR(10) NOT NULL,
  street_pre_direction VARCHAR(2),
  street_name VARCHAR(50) NOT NULL,
  street_suffix VARCHAR(20) NOT NULL,
  street_post_direction VARCHAR(2),
  secondary_designator VARCHAR(10),
  secondary_number VARCHAR(10),
  city VARCHAR(50) NOT NULL,
  state_or_region VARCHAR(50) NOT NULL,
  zip_or_postal_code VARCHAR(20) NOT NULL,
  latitude NUMERIC(8, 6),
  longitude NUMERIC(9, 6),
  admin_division VARCHAR(50) NOT NULL,
  land_use_type VARCHAR(50) NOT NULL,
  area_sq_ft INTEGER,
  nr_bedrooms SMALLINT,
  nr_bathrooms NUMERIC(4, 2),
  total_area_sq_ft INTEGER,
  created_on timestamptz NOT NULL,
  last_updated_on timestamptz NOT NULL
)

