ALTER TABLE tecido_cores
    ADD COLUMN IF NOT EXISTS custo_override NUMERIC(12, 2);

ALTER TABLE tecido_estampas
    ADD COLUMN IF NOT EXISTS custo_override NUMERIC(12, 2);
