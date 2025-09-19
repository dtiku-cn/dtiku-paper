create type pay_from as enum('alipay', 'wechat', 'qq');
create type order_level as enum('monthly', 'quarterly', 'half_year', 'annual');
create type order_status as enum('created', 'paid', 'canceled', 'refunded');
create table if not exists pay_order(
    id serial primary key,
    user_id integer not null,
    level order_level not null,
    pay_from pay_from not null,
    resp jsonb default null,
    confirm timestamp default null,
    status order_status not null,
    created timestamp not null,
    modified timestamp not null
);