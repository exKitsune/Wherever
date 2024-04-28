package com.fruit.wherever;

import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.SQLException;
import android.database.sqlite.SQLiteDatabase;

import static com.fruit.wherever.DatabaseHelper.TABLE_NAME;

public class DBManager {

    private DatabaseHelper dbHelper;

    private Context context;

    private SQLiteDatabase database;

    public DBManager(Context c) {
        context = c;
    }

    public DBManager open() throws SQLException {
        dbHelper = new DatabaseHelper(context);
        database = dbHelper.getWritableDatabase();
        return this;
    }

    public void close() {
        dbHelper.close();
    }

    public void insert(String host, String component) {
        host = host.toLowerCase();
        ContentValues contentValue = new ContentValues();
        contentValue.put(DatabaseHelper.HOST, host);
        contentValue.put(DatabaseHelper.COMPONENT, component);
        database.insert(TABLE_NAME, null, contentValue);
    }

    public Cursor fetch(String host) {
        host = host.toLowerCase();
        String[] columns = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT };
        String selection = DatabaseHelper.HOST + " = ?";
        String[] selectionArgs = { host };

        Cursor cursor = database.query(TABLE_NAME, columns, selection, selectionArgs, null, null, null);

        return cursor;
    }

    public int update(String host, String component) {
        host = host.toLowerCase();
        ContentValues contentValues = new ContentValues();
        contentValues.put(DatabaseHelper.HOST, host);
        contentValues.put(DatabaseHelper.COMPONENT, component);
        int i = database.update(TABLE_NAME, contentValues, DatabaseHelper.HOST + " = " + host, null);
        return i;
    }

    public void delete(String host) {
        host = host.toLowerCase();
        database.delete(TABLE_NAME, DatabaseHelper.HOST + "=" + host, null);
    }

    public void drop() {
        dbHelper.onDrop(database);
    }

}