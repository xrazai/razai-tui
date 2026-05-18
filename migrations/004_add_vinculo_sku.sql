ALTER TABLE tecido_cores
    ADD COLUMN IF NOT EXISTS sku TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS tecido_cores_sku_key ON tecido_cores (sku);
