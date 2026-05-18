# Banco local

Este projeto esta preparado para usar PostgreSQL local via Docker.

## Subir o banco

Instale o Docker Desktop e, depois de abrir o Docker, rode:

```powershell
docker compose up -d
```

## Parar o banco

```powershell
docker compose down
```

## URL local

Copie `.env.example` para `.env`.

```env
DATABASE_URL=postgres://razai:razai_dev@localhost:5432/razai_tui
```

## Dados

As migrations ficam em `migrations/`. Na primeira vez que o container sobe, o Postgres executa os arquivos `.sql` dessa pasta.

Se precisar recriar o banco do zero:

```powershell
docker compose down -v
docker compose up -d
```

O `-v` apaga o volume de dados local.
