#import <Foundation/Foundation.h>

#define MACRO_STRING_(m) #m
#define MACRO_STRING(m) @MACRO_STRING_(m)

NS_ASSUME_NONNULL_BEGIN

static NSString * const AppBundleIdentifier = MACRO_STRING(APP_BUNDLE_IDENTIFIER);
static NSString * const AppGroupIdentifier = MACRO_STRING(APP_GROUP_IDENTIFIER);
static NSString * const NetworkExtensionBundleIdentifier = MACRO_STRING(NETWORK_EXTENSION_BUNDLE_IDENTIFIER);

NS_ASSUME_NONNULL_END
