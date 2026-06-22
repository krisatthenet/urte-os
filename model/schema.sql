-- URTE DB entity schema
-- Generated from the urtecore Capella MBSE model (DataPkg "Data":
-- packages "DATA schema package" and "Data Vault").
-- All model associations are 1..1; properties -> foreign keys, services -> comments.
-- Tables are ordered so referenced tables are created before referencing ones.

-- ---- Data Vault (leaf / referenced entities first) ----------------------

CREATE TABLE divinity_buffer (        -- service: Block
    id UUID PRIMARY KEY
);

CREATE TABLE trinity_buffer (         -- service: Block II
    id UUID PRIMARY KEY
);

CREATE TABLE class_1_2 (
    id UUID PRIMARY KEY
);

CREATE TABLE heap_controler (         -- service: Index
    id                 UUID PRIMARY KEY,
    divinity_buffer_id UUID NOT NULL REFERENCES divinity_buffer(id),
    trinity_buffer_id  UUID NOT NULL REFERENCES trinity_buffer(id)
);

-- ---- DATA schema package ------------------------------------------------

CREATE TABLE stream_ops (             -- service: DataMesh
    id                 UUID PRIMARY KEY,
    heap_controler_id  UUID NOT NULL REFERENCES heap_controler(id),
    divinity_buffer_id UUID NOT NULL REFERENCES divinity_buffer(id),
    trinity_buffer_id  UUID NOT NULL REFERENCES trinity_buffer(id)
);

CREATE TABLE shuffler (               -- service: Alligment
    id            UUID PRIMARY KEY,
    stream_ops_id UUID NOT NULL REFERENCES stream_ops(id)
);

CREATE TABLE heap_2 (                 -- service: Buffer
    id            UUID PRIMARY KEY,
    shuffler_id   UUID NOT NULL REFERENCES shuffler(id),
    stream_ops_id UUID NOT NULL REFERENCES stream_ops(id)
);

CREATE TABLE heap_1 (                 -- service: Cache
    id            UUID PRIMARY KEY,
    heap2_id      UUID NOT NULL REFERENCES heap_2(id),
    stream_ops_id UUID NOT NULL REFERENCES stream_ops(id)
);

CREATE TABLE filter (                 -- service: Patterning; summary: Filter
    id            UUID PRIMARY KEY,
    heap1_id      UUID NOT NULL REFERENCES heap_1(id),
    stream_ops_id UUID NOT NULL REFERENCES stream_ops(id)
);
