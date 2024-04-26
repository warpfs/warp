#import <CoreFoundation/CoreFoundation.h>
#import <Security/Security.h>

#import <string.h>

int default_store_store_key(const UInt8 *id, const UInt8 *key, const char *tag) {
    CFMutableDictionaryRef attrs;
    CFDataRef bin;
    CFStringRef str;
    OSStatus status;

    // Setup attributes,
    attrs = CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);

    CFDictionarySetValue(attrs, kSecClass, kSecClassKey);

    bin = CFDataCreateWithBytesNoCopy(NULL, key, 16, kCFAllocatorNull);
    CFDictionarySetValue(attrs, kSecValueData, bin);
    CFRelease(bin);

    str = CFStringCreateWithCStringNoCopy(NULL, "Warp File Key", kCFStringEncodingUTF8, kCFAllocatorNull);
    CFDictionarySetValue(attrs, kSecAttrLabel, str);
    CFRelease(str);

    bin = CFDataCreateWithBytesNoCopy(NULL, id, 16, kCFAllocatorNull);
    CFDictionarySetValue(attrs, kSecAttrApplicationLabel, bin);
    CFRelease(bin);

    bin = CFDataCreateWithBytesNoCopy(NULL, (const UInt8 *)tag, strlen(tag), kCFAllocatorNull);
    CFDictionarySetValue(attrs, kSecAttrApplicationTag, bin);
    CFRelease(bin);

    CFDictionarySetValue(attrs, kSecAttrIsPermanent, kCFBooleanTrue);
    CFDictionarySetValue(attrs, kSecAttrSynchronizable, kCFBooleanTrue);
    CFDictionarySetValue(attrs, kSecUseDataProtectionKeychain, kCFBooleanTrue);
    CFDictionarySetValue(attrs, kSecAttrAccessible, kSecAttrAccessibleAfterFirstUnlock);
    CFDictionarySetValue(attrs, kSecAttrKeyClass, kSecAttrKeyClassSymmetric);

    // Store key.
    status = SecItemAdd(attrs, nil);
    CFRelease(attrs);

    return status;
}
