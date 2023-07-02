create table rating (
    id bigint not null primary key,
    username varchar(255) not null,
    game_id bigint not null,
    rating integer not null,
    created_at timestamp not null,
    updated_at timestamp,
    constraint unique_rating_per_game unique (username, game_id)
);

create table game (
    id bigint not null primary key,
    name varchar(255) not null,
    year integer not null,
    created_at timestamp not null,
    updated_at timestamp,
    constraint unique_game_name_year unique (name, year)
);

create table review (
    id bigint not null primary key,
    username varchar(255) not null,
    game_id bigint not null,
    review_description text not null,
    rating_id bigint not null,
    created_at timestamp not null,
    updated_at timestamp,
    constraint fk_review_rating_id foreign key (rating_id) references rating (id),
    constraint unique_review_per_game unique (username, game_id)
);

alter table rating add constraint fk_rating_game_id foreign key (game_id) references game (id);