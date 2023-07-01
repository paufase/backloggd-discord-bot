create table rating (
    id integer not null primary key,
    username varchar(255) not null,
    game_id integer not null,
    rating integer not null,
    created_at timestamp not null,
    updated_at timestamp not null,
    constraint fk_game_id foreign key (game_id) references game (id),
    constraint unique_rating_per_game unique (username, game_id)
);

create table game (
    id integer not null primary key,
    name varchar(255) not null,
    year integer not null,
    created_at timestamp not null,
    updated_at timestamp not null
);
