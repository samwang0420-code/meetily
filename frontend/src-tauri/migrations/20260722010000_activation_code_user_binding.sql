ALTER TABLE activation_codes ADD COLUMN bound_user_id INTEGER;
CREATE INDEX idx_activation_codes_bound_user_id ON activation_codes(bound_user_id);
UPDATE activation_codes
SET bound_user_id = used_by_user_id
WHERE bound_user_id IS NULL AND used_by_user_id IS NOT NULL;
