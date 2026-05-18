CREATE TABLE IF NOT EXISTS estampas (
    id BIGSERIAL PRIMARY KEY,
    nome TEXT NOT NULL,
    sku TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tecido_estampas (
    id BIGSERIAL PRIMARY KEY,
    tecido_id BIGINT NOT NULL REFERENCES tecidos(id) ON DELETE CASCADE,
    estampa_id BIGINT NOT NULL REFERENCES estampas(id) ON DELETE RESTRICT,
    sku TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tecido_id, estampa_id)
);

CREATE INDEX IF NOT EXISTS idx_tecido_estampas_tecido_id ON tecido_estampas(tecido_id);
CREATE INDEX IF NOT EXISTS idx_tecido_estampas_estampa_id ON tecido_estampas(estampa_id);
