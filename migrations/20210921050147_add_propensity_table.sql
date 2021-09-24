-- Create Propensity Table
CREATE TABLE Propensities (
  id serial PRIMARY KEY,
  apn VARCHAR(50) UNIQUE NOT NULL,
  score SMALLINT NOT NULL,
  created_on timestamptz NOT NULL,
  last_updated_on timestamptz NOT NULL
)