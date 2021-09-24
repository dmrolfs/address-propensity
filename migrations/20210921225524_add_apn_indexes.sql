-- Add migration script here
CREATE INDEX idx_property_apn ON Properties(apn);
CREATE INDEX idx_propensity_apn ON Propensities(apn)
