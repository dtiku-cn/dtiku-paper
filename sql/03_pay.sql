create type pay_from as enum('alipay', 'wechat', 'qq');
create type order_level as enum('monthly', 'quarterly', 'half_year', 'annual');
create table if not exists pay_order(
    id serial primary key,
    user_id integer not null,
    level order_level not null,
    pay_from pay_from not null,
    confirm timestamp default null,
    created timestamp not null,
    modified timestamp not null
);