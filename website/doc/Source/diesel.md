# Diesel

目前，Diesel 1.0不支持异步操作，但是可以将`kayrx::fiber`线程池系统用作数据库接口api

创建一个简单的数据库api，可以将新的用户行插入`SQLite`表, 相同的方法可以用于其他数据库

> `awesome`目录中提供了完整的[示例](https://github.com/kayrx/awesome/blob/master/examples/async-sqlite) :  kayrx + r2d2 + rusqlite
> 