DROP TABLE IF EXISTS device_color;
CREATE TABLE device_color(
  color_id integer PRIMARY KEY GENERATED ALWAYS	AS IDENTITY,
  color_value text UNIQUE
);

DROP TABLE IF EXISTS web_device;
CREATE TABLE web_device(
  web_device_id integer NOT NULL PRIMARY KEY,
  web_device_name text NOT NULL,
  visible bool NOT NULL DEFAULT false,
  colored bool NOT NULL DEFAULT false, 
  icon text
);

DROP TABLE IF EXISTS device;
CREATE TABLE device(
  device_id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  color_id integer,
  color_visible bool,
  web_device_id integer NOT NULL
);
  
-- data
INSERT INTO web_device 
VALUES (1,'Mifare'), 
	   (5, 'Транспортная карта'),
	   (6, 'Банковская карта'),
	   (7, 'Соц. карта учащегося'),
	   (8, 'Соц. карта москвича'),
	   (9, 'Браслет (Mifare)'),
	   (10, 'Часы (Mifare)'),
	   (11, 'Брелок (Mifare)'),
	   (12, 'Тройка-москвёнок карта'),
	   (13, 'Тройка-москвёнок браслет'),
	   (14, 'Тройка-москвёнок брелок'), 
	   (15, 'Фитнес-браслет'),
	   (16, 'Смарт-кольцо');


INSERT INTO device_color(color_value)
VALUES ('ff0000'),
	   ('ffff00'),
	   ('39ff00'),
	   ('0900ff');
