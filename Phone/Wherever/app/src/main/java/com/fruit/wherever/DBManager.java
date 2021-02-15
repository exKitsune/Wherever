package com.fruit.wherever;

import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.SQLException;
import android.database.sqlite.SQLiteDatabase;
import android.provider.ContactsContract;
import android.util.Log;

import static com.fruit.wherever.DatabaseHelper.TABLE_NAME;

public class DBManager {

    private static DatabaseHelper dbHelper = null;

    private static Context context;

    private static SQLiteDatabase database;

    private static DBManager instance = null;

    public DBManager(Context c) {
        context = c;
    }

    public static DBManager getInstance(Context c){
        if (instance == null){
            instance = new DBManager(c);
        }
        return instance;
    }

    public DBManager open() throws SQLException {
        dbHelper = new DatabaseHelper(context);
        database = dbHelper.getWritableDatabase();
        return this;
    }

    public void close() {
        dbHelper.close();
    }

    public void insert(String host, String component, long accessed ) {
        host = host.toLowerCase();
        ContentValues contentValue = new ContentValues();
        contentValue.put(DatabaseHelper.HOST, host);
        contentValue.put(DatabaseHelper.COMPONENT, component);
        contentValue.put(DatabaseHelper.ACCESSED, accessed);
        database.insert(TABLE_NAME, null, contentValue);
    }

    public Cursor fetch(String host) {
        host = host.toLowerCase();
        String[] columns = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT, DatabaseHelper.ACCESSED };
        String selection = DatabaseHelper.HOST + " = ?";
        String[] selectionArgs = { host };

        Cursor cursor = database.query(TABLE_NAME, columns, selection, selectionArgs, null, null, null);

        return cursor;
    }

    public Cursor fetchAll() {
        String[] columns = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT, DatabaseHelper.ACCESSED };
        String selection = DatabaseHelper.HOST + " != ? AND " + DatabaseHelper.HOST + " != ?";
        String[] selectionArgs = { "default_browser", "potential_browsers" };
        Cursor cursor = database.query(TABLE_NAME, columns, selection, selectionArgs, null, null, DatabaseHelper.ACCESSED + " DESC");

        return cursor;
    }

    public int update(String host, String component, long accessed) {
        host = host.toLowerCase();
        ContentValues contentValues = new ContentValues();
        //contentValues.put(DatabaseHelper.HOST, host);
        contentValues.put(DatabaseHelper.COMPONENT, component);
        contentValues.put(DatabaseHelper.ACCESSED, accessed);
        String selection = DatabaseHelper.HOST + " LIKE ?";
        String[] selectionArgs = { host };

        int i = database.update(TABLE_NAME, contentValues, selection, selectionArgs);
        return i;
    }

    public int put(String host, String component, long accessed) {
        Cursor cursor = fetch(host);
        if (cursor.getCount() == 0) {
            insert(host, component, accessed);
            return 0;
        } else {
            return update(host, component, accessed);
        }
    }

    public void delete(String host) {
        host = host.toLowerCase();
        String selection = DatabaseHelper.HOST + " LIKE ?";
        String[] selectionArgs = { host };
        database.delete(TABLE_NAME, selection, selectionArgs);
    }

    public void drop() {
        dbHelper.onDrop(database);
    }

}