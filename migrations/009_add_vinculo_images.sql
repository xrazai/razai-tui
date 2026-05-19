ALTER TABLE tecido_cores
    ADD COLUMN IF NOT EXISTS imagem_original BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_brand BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_modelo BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_alternativa BYTEA;

ALTER TABLE tecido_estampas
    ADD COLUMN IF NOT EXISTS imagem_original BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_brand BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_modelo BYTEA,
    ADD COLUMN IF NOT EXISTS imagem_alternativa BYTEA;
