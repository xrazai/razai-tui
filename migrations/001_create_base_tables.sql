CREATE TABLE IF NOT EXISTS tecidos (
    id BIGSERIAL PRIMARY KEY,
    nome TEXT NOT NULL,
    sku VARCHAR(4) NOT NULL UNIQUE,
    composicao TEXT NOT NULL,
    largura_m NUMERIC(8, 3) NOT NULL,
    rendimento_m_kg NUMERIC(10, 2),
    gramatura_linear_g_m INTEGER,
    gramatura_g_m2 INTEGER,
    tipo TEXT NOT NULL DEFAULT 'Selecione',
    transparencia TEXT NOT NULL DEFAULT 'Selecione',
    elasticidade TEXT NOT NULL DEFAULT 'Selecione',
    acabamento TEXT NOT NULL DEFAULT 'Selecione',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cores (
    id BIGSERIAL PRIMARY KEY,
    nome TEXT NOT NULL,
    sku TEXT UNIQUE,
    codigo_hex CHAR(7),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tecido_cores (
    id BIGSERIAL PRIMARY KEY,
    tecido_id BIGINT NOT NULL REFERENCES tecidos(id) ON DELETE CASCADE,
    cor_id BIGINT NOT NULL REFERENCES cores(id) ON DELETE RESTRICT,
    sku TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tecido_id, cor_id)
);

CREATE INDEX IF NOT EXISTS idx_tecido_cores_tecido_id ON tecido_cores(tecido_id);
CREATE INDEX IF NOT EXISTS idx_tecido_cores_cor_id ON tecido_cores(cor_id);
