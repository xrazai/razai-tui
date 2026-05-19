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

## Variaveis locais

Copie `.env.example` para `.env` e mantenha o `.env` fora do Git.

O valor de `DATABASE_URL` no exemplo aponta para o Postgres local criado pelo `docker-compose.yml`. Se voce trocar usuario, senha, porta ou nome do banco no Docker, atualize tambem o `.env` local.

Nao documente chaves reais de API ou senhas pessoais no README, docs ou commits. `OPENROUTER_API_KEY` deve existir apenas no `.env` local.

## Dados

As migrations ficam em `migrations/`. Na primeira vez que o container sobe, o Postgres executa os arquivos `.sql` dessa pasta.

Tabelas principais:

- `tecidos`: tecidos cadastrados, SKU, composicao, largura, tipo e gramaturas.
- `cores`: cores cadastradas, hexadecimal, swatch derivado e SKU.
- `estampas`: estampas cadastradas e SKU.
- `tecido_cores`: vinculos de tecidos lisos com cores.
- `tecido_estampas`: vinculos de tecidos estampados com estampas.
- `configuracoes`: configuracoes locais persistidas no banco, como impressora de recibos.

O app tambem garante em runtime as tabelas `configuracoes`, `estampas` e `tecido_estampas`, porque bancos locais antigos podem ter sido criados antes dessas migrations.

## Configuracoes

Configuracoes usam pares `chave`/`valor`.

| Chave | Uso |
| --- | --- |
| `receipt_printer` | Nome da impressora de recibos 80mm selecionada em `Configuracoes`. |

Se precisar recriar o banco do zero:

```powershell
docker compose down -v
docker compose up -d
```

O `-v` apaga o volume de dados local.
