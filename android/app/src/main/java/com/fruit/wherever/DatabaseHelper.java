package com.fruit.wherever;

import android.content.Context;
import android.database.sqlite.SQLiteDatabase;
import android.database.sqlite.SQLiteOpenHelper;
import android.provider.ContactsContract;

public class DatabaseHelper extends SQLiteOpenHelper {
    private static DatabaseHelper instance = null;
    // Table Name
    public static final String TABLE_NAME = "USERPREF";

    // Table columns
    public static final String HOST = "_id";
    public static final String COMPONENT = "component";
    public static final String ACCESSED = "accessed";

    // Database Information
    static final String DB_NAME = "WHEREVER_USER_PREF.DB";

    // database version
    static final int DB_VERSION = 1;

    // Creating table query
    private static final String CREATE_TABLE = "create table " + TABLE_NAME + "(" + HOST + " TEXT NOT NULL PRIMARY KEY, " + COMPONENT + " TEXT NOT NULL, " + ACCESSED + " BIGINT NOT NULL);";

    public DatabaseHelper(Context context) {
        super(context, DB_NAME, null, DB_VERSION);
    }

    @Override
    public void onCreate(SQLiteDatabase db) {
        db.execSQL(CREATE_TABLE);
    }

    @Override
    public void onUpgrade(SQLiteDatabase db, int oldVersion, int newVersion) {
        db.execSQL("DROP TABLE IF EXISTS " + TABLE_NAME);
        onCreate(db);
    }

    public void onDrop(SQLiteDatabase db) {
        db.execSQL("DROP TABLE IF EXISTS " + TABLE_NAME);
        onCreate(db);
    }
}