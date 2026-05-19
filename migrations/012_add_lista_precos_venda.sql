ALTER TABLE tecidos
    ADD COLUMN IF NOT EXISTS preco_atacado NUMERIC(12, 2),
    ADD COLUMN IF NOT EXISTS preco_varejo NUMERIC(12, 2);

ALTER TABLE tecido_cores
    ADD COLUMN IF NOT EXISTS preco_atacado_override NUMERIC(12, 2),
    ADD COLUMN IF NOT EXISTS preco_varejo_override NUMERIC(12, 2);

ALTER TABLE tecido_estampas
    ADD COLUMN IF NOT EXISTS preco_atacado_override NUMERIC(12, 2),
    ADD COLUMN IF NOT EXISTS preco_varejo_override NUMERIC(12, 2);
