package com.web3analytics.serialization;

import com.web3analytics.errors.DeserializationException;
import org.apache.avro.generic.GenericRecord;

final class AvroFieldReader {

    private AvroFieldReader() {
    }

    static String string(GenericRecord record, String fieldName) {
        return required(record, fieldName).toString();
    }

    static String nullableString(GenericRecord record, String fieldName) {
        Object value = record.get(fieldName);
        return value == null ? null : value.toString();
    }

    static long longValue(GenericRecord record, String fieldName) {
        Object value = required(record, fieldName);
        if (value instanceof Number number) {
            return number.longValue();
        }
        throw new DeserializationException("Field '" + fieldName + "' expected long but was " + value.getClass().getSimpleName());
    }

    static int intValue(GenericRecord record, String fieldName) {
        Object value = required(record, fieldName);
        if (value instanceof Number number) {
            return number.intValue();
        }
        throw new DeserializationException("Field '" + fieldName + "' expected int but was " + value.getClass().getSimpleName());
    }

    static double doubleValue(GenericRecord record, String fieldName) {
        Object value = required(record, fieldName);
        if (value instanceof Number number) {
            return number.doubleValue();
        }
        throw new DeserializationException("Field '" + fieldName + "' expected double but was " + value.getClass().getSimpleName());
    }

    static Double nullableDouble(GenericRecord record, String fieldName) {
        Object value = record.get(fieldName);
        if (value == null) {
            return null;
        }
        if (value instanceof Number number) {
            return number.doubleValue();
        }
        throw new DeserializationException("Field '" + fieldName + "' expected double but was " + value.getClass().getSimpleName());
    }

    private static Object required(GenericRecord record, String fieldName) {
        Object value = record.get(fieldName);
        if (value == null) {
            throw new DeserializationException("Required field is null: " + fieldName);
        }
        return value;
    }
}
